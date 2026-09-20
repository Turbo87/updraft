use crate::driver::DriverHandle;
use std::{path::PathBuf, sync::Arc};
use updraft_core::{ChangeTask, GetTask, Task, TaskCommand};

pub struct TaskFile {
    path: PathBuf,
    changes: tokio::sync::Mutex<()>,
}

impl TaskFile {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            path: directory.join("task.json"),
            changes: Default::default(),
        }
    }

    pub fn load(&self) -> std::io::Result<Task> {
        match std::fs::File::open(&self.path) {
            Ok(file) => Ok(serde_json::from_reader(std::io::BufReader::new(file))?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Task::default()),
            Err(error) => Err(error),
        }
    }

    pub fn start(
        self: &Arc<Self>,
        handle: DriverHandle,
        navigation: crate::navigation::NavigationFile,
    ) {
        let (changes, mut receiver) = tokio::sync::watch::channel(());
        let initial = std::sync::atomic::AtomicBool::new(true);
        handle.subscribe(Box::new(move |topic| {
            if matches!(topic, updraft_core::Topic::Task(_))
                && !initial.swap(false, std::sync::atomic::Ordering::Relaxed)
            {
                changes.send_replace(());
            }
            !changes.is_closed()
        }));
        let file = self.clone();
        tauri::async_runtime::spawn(async move {
            while receiver.changed().await.is_ok() {
                if let Err(error) = persist(&file, &navigation, &handle).await {
                    tracing::warn!(%error, "Task persistence worker stopped");
                    break;
                }
            }
        });
    }

    pub async fn save(&self, handle: &DriverHandle) -> Result<bool, String> {
        let _guard = self.changes.lock().await;
        let task = handle
            .send(GetTask)
            .await
            .map_err(|error| error.to_string())?;
        let path = self.path.clone();
        match tauri::async_runtime::spawn_blocking(move || {
            crate::navigation::save_target_file(path, &task)
        })
        .await
        {
            Ok(Ok(())) => Ok(true),
            error => {
                tracing::warn!(?error, "Could not save task");
                Ok(false)
            }
        }
    }
}

#[tauri::command]
pub async fn change_task(
    command: TaskCommand,
    handle: tauri::State<'_, DriverHandle>,
    file: tauri::State<'_, Arc<TaskFile>>,
    navigation: tauri::State<'_, crate::navigation::NavigationFile>,
) -> Result<bool, String> {
    handle
        .send(ChangeTask(command))
        .await
        .map_err(|error| error.to_string())?
        .map_err(str::to_owned)?;
    persist(&file, &navigation, &handle).await
}

#[tauri::command]
pub async fn save_task(
    handle: tauri::State<'_, DriverHandle>,
    file: tauri::State<'_, Arc<TaskFile>>,
    navigation: tauri::State<'_, crate::navigation::NavigationFile>,
) -> Result<bool, String> {
    persist(&file, &navigation, &handle).await
}

async fn persist(
    file: &TaskFile,
    navigation: &crate::navigation::NavigationFile,
    handle: &DriverHandle,
) -> Result<bool, String> {
    let task_saved = file.save(handle).await?;
    let navigation_saved = navigation.save_current(handle).await?;
    let saved = task_saved && navigation_saved;
    handle
        .send(updraft_core::SetTaskSaveFailed(!saved))
        .await
        .map_err(|error| error.to_string())?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{invoke, spawn_driver};
    use claims::assert_ok;
    use serde_json::json;
    use tauri::Manager;

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn commands_save_task_progress_and_primary_selection() {
        let directory = assert_ok!(tempfile::tempdir());
        let app = tauri::test::mock_builder()
            .manage(Arc::new(TaskFile::new(directory.path().to_owned())))
            .manage(crate::navigation::NavigationFile::new(
                directory.path().to_owned(),
            ))
            .manage(spawn_driver(
                Default::default(),
                updraft_core::AirspaceState::none_at_startup(),
            ))
            .invoke_handler(tauri::generate_handler![change_task, save_task])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        for name in ["Start", "Finish"] {
            assert_eq!(
                assert_ok!(invoke(
                    &app,
                    "change_task",
                    json!({"command":{"type":"add","target":{"type":"waypoint","name":name,"latitudeDegrees":50.,"longitudeDegrees":6.,"elevationMeters":100.}}})
                )),
                json!(true)
            );
        }
        assert_eq!(
            assert_ok!(invoke(
                &app,
                "change_task",
                json!({"command":{"type":"select","id":1}})
            )),
            json!(true)
        );
        let task = assert_ok!(app.state::<Arc<TaskFile>>().load());
        assert_eq!(task.current, Some(1));
        assert_eq!(task.status, updraft_core::TaskStatus::Running);
        assert_eq!(
            assert_ok!(app.state::<crate::navigation::NavigationFile>().load()),
            Some(updraft_core::NavigationTarget::Task)
        );
        assert_eq!(
            assert_ok!(invoke(
                &app,
                "change_task",
                json!({"command":{"type":"stop"}})
            )),
            json!(true)
        );
        assert_eq!(
            assert_ok!(app.state::<Arc<TaskFile>>().load()).status,
            updraft_core::TaskStatus::Stopped
        );
        assert_eq!(
            assert_ok!(app.state::<crate::navigation::NavigationFile>().load()),
            None
        );
        let file = app.state::<Arc<TaskFile>>();
        assert_ok!(std::fs::remove_file(&file.path));
        assert_ok!(std::fs::create_dir(&file.path));
        assert_eq!(
            assert_ok!(invoke(
                &app,
                "change_task",
                json!({"command":{"type":"add","target":{"type":"waypoint","name":"Extra","latitudeDegrees":51.,"longitudeDegrees":6.,"elevationMeters":100.}}})
            )),
            json!(false)
        );
        assert_ok!(std::fs::remove_dir(&file.path));
        assert_eq!(
            assert_ok!(invoke(&app, "save_task", json!({}))),
            json!(true)
        );
        assert_eq!(assert_ok!(file.load()).points.len(), 3);
        let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
        assert!(String::from_utf8_lossy(&logs).contains("Could not save task"));
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn background_changes_save_without_a_frontend_command() {
        let directory = assert_ok!(tempfile::tempdir());
        let file = Arc::new(TaskFile::new(directory.path().to_owned()));
        let handle = spawn_driver(
            Default::default(),
            updraft_core::AirspaceState::none_at_startup(),
        );
        let navigation = crate::navigation::NavigationFile::new(directory.path().to_owned());
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic| {
            if matches!(topic, updraft_core::Topic::TaskSaveFailed(false)) {
                return sender.send(()).is_ok();
            }
            true
        }));
        file.start(handle.clone(), navigation);
        assert_ok!(
            handle
                .send(ChangeTask(TaskCommand::Add {
                    target: updraft_core::NavigationTarget::Waypoint {
                        name: "Start".into(),
                        latitude_degrees: 50.,
                        longitude_degrees: 6.,
                        elevation_meters: 100.
                    }
                }))
                .await
        )
        .unwrap();
        assert_ok!(
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while receiver.recv().await.is_some() {
                    if file.load().is_ok_and(|task| task.points.len() == 1) {
                        return;
                    }
                }
                panic!("Task persistence notifications stopped");
            })
            .await
        );
        assert_eq!(assert_ok!(file.load()).points[0].id, 0);
    }
}

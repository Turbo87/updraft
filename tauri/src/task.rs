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
    let task_saved = file.save(&handle).await?;
    Ok(navigation.save_current(&handle).await? && task_saved)
}

#[tauri::command]
pub async fn save_task(
    handle: tauri::State<'_, DriverHandle>,
    file: tauri::State<'_, Arc<TaskFile>>,
    navigation: tauri::State<'_, crate::navigation::NavigationFile>,
) -> Result<bool, String> {
    let saved = file.save(&handle).await?;
    Ok(navigation.save_current(&handle).await? && saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{invoke, spawn_driver};
    use claims::assert_ok;
    use serde_json::json;
    use tauri::Manager;

    #[tokio::test(flavor = "multi_thread")]
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
    }
}

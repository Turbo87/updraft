use super::{arrival_resource::ArrivalResource, arrivals::ArrivalCalculator};
use crate::driver::DriverHandle;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{
    Manager,
    http::{Response, StatusCode},
    ipc::Channel,
};
use tokio::sync::watch;
use updraft_geo::BoundingBox;
use updraft_units::Angle;

#[derive(Default)]
pub struct ArrivalStreams(Arc<Mutex<BTreeMap<String, Stream>>>);

struct Stream {
    viewport: watch::Sender<BoundingBox>,
    results: watch::Receiver<Option<ArrivalResource>>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ArrivalNotification {
    Ready { generation: u64, revision: u64 },
    Failed,
}

impl ArrivalStreams {
    fn start(
        &self,
        driver: DriverHandle,
        bounds: BoundingBox,
        channel: Channel<ArrivalNotification>,
    ) -> String {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
        let ArrivalCalculator {
            viewport,
            mut results,
            task,
        } = ArrivalCalculator::spawn(driver, bounds);
        self.0.lock().unwrap().insert(
            id.clone(),
            Stream {
                viewport,
                results: results.clone(),
            },
        );
        let streams = self.0.clone();
        let stream_id = id.clone();
        tokio::spawn(async move {
            let mut revision = 0;
            while results.changed().await.is_ok() {
                let current = results.borrow_and_update();
                let Some(resource) = current.as_ref() else {
                    continue;
                };
                revision += 1;
                let update = ArrivalNotification::Ready {
                    generation: resource.generation,
                    revision,
                };
                if channel.send(update).is_err() {
                    break;
                }
            }
            streams.lock().unwrap().remove(&stream_id);
            let outcome = match task.await {
                Ok(result) => result,
                Err(error) => Err(error.into()),
            };
            if let Err(error) = outcome {
                tracing::error!(%error, "Arrival worker stopped unexpectedly");
                let _ = channel.send(ArrivalNotification::Failed);
            }
        });
        id
    }

    fn response(&self, id: &str) -> Response<Vec<u8>> {
        let streams = self.0.lock().unwrap();
        let (status, body) = match streams.get(id) {
            Some(stream) => match stream.results.borrow().as_ref() {
                Some(resource) => (StatusCode::OK, resource.body.clone()),
                None => (StatusCode::SERVICE_UNAVAILABLE, Vec::new()),
            },
            None => (StatusCode::NOT_FOUND, Vec::new()),
        };
        Response::builder()
            .status(status)
            .header("content-type", "application/geo+json")
            .header("cache-control", "no-store")
            .body(body)
            .unwrap()
    }
}

/// Bounds use `[west, south, east, north]`. Longitudes may span multiple world copies.
/// Pass `east >= west`, including when the viewport crosses the antimeridian.
fn viewport([west, south, east, north]: [f64; 4]) -> Result<BoundingBox, &'static str> {
    let width = east - west;
    if ![west, south, east, north, width]
        .iter()
        .all(|value| value.is_finite())
        || south < -90.
        || north > 90.
        || south > north
        || width < 0.
    {
        return Err("Invalid arrival viewport bounds");
    }
    let (west, east) = if width >= 360. {
        (Angle::from_degrees(-180.), Angle::from_degrees(180.))
    } else {
        (
            Angle::from_degrees(west).normalized_signed(),
            Angle::from_degrees(east).normalized_signed(),
        )
    };
    Ok(BoundingBox::new(
        Angle::from_degrees(south),
        Angle::from_degrees(north),
        west,
        east,
    ))
}

#[tauri::command]
pub async fn start_arrivals(
    bounds: [f64; 4],
    channel: Channel<ArrivalNotification>,
    state: tauri::State<'_, ArrivalStreams>,
    driver: tauri::State<'_, DriverHandle>,
) -> Result<String, &'static str> {
    Ok(state.start(driver.inner().clone(), viewport(bounds)?, channel))
}

#[tauri::command]
pub fn update_arrival_viewport(
    id: String,
    bounds: [f64; 4],
    state: tauri::State<'_, ArrivalStreams>,
) -> Result<(), &'static str> {
    let bounds = viewport(bounds)?;
    let streams = state.0.lock().unwrap();
    let stream = streams.get(&id).ok_or("Arrival subscription is closed")?;
    stream
        .viewport
        .send(bounds)
        .map_err(|_| "Arrival worker stopped")
}

#[tauri::command]
pub fn stop_arrivals(id: String, state: tauri::State<'_, ArrivalStreams>) {
    state.0.lock().unwrap().remove(&id);
}

pub async fn arrival_resource_response<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    id: String,
) -> Response<Vec<u8>> {
    app.state::<ArrivalStreams>().response(&id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::tests::spawn;
    use claims::{assert_err, assert_ok, assert_some};
    use serde_json::{Value, json};
    use std::time::Duration;
    use tauri::Manager;
    use updraft_core::SettingsSnapshot;

    fn invoke(
        window: &tauri::WebviewWindow<tauri::test::MockRuntime>,
        command: &str,
        body: Value,
    ) -> Result<Value, Value> {
        let request = tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        };
        tauri::test::get_ipc_response(window, request).map(|body| body.deserialize().unwrap())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn viewport_validation_uses_the_ipc_boundary() {
        let driver = spawn(
            SettingsSnapshot::default(),
            Box::new(|_, _, _| unreachable!()),
            Box::new(|_| {}),
            Duration::from_secs(60),
        );
        let app = tauri::test::mock_builder()
            .manage(ArrivalStreams::default())
            .manage(driver.handle.clone())
            .invoke_handler(tauri::generate_handler![
                start_arrivals,
                update_arrival_viewport,
                stop_arrivals
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let (sender, receiver) = watch::channel(viewport([0., 0., 1., 1.]).unwrap());
        let (_, results) = watch::channel(None);
        let streams = app.state::<ArrivalStreams>();
        streams.0.lock().unwrap().insert(
            "test".into(),
            Stream {
                viewport: sender,
                results,
            },
        );
        assert_eq!(
            streams.response("test").status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        for (bounds, valid) in [
            (json!([170, -10, 190, 10]), true),
            (json!([-200, -90, 200, 90]), true),
            (json!([0, 10, 1, -10]), false),
            (json!([0, -91, 1, 0]), false),
            (json!([0, 0, 1, 91]), false),
            (json!([10, 0, 0, 1]), false),
            (json!([0, 0, null, 1]), false),
            (json!([-1e308, 0, 1e308, 1]), false),
        ] {
            let body = json!({"id":"test", "bounds":bounds});
            let response = invoke(&window, "update_arrival_viewport", body);
            if valid {
                assert_ok!(response);
                let dateline = updraft_geo::LatLon::from_degrees(0., 180.);
                assert!(receiver.borrow().contains(dateline));
            } else {
                assert_err!(response);
            }
        }
        assert_eq!(receiver.borrow().longitude_span().as_degrees(), 360.);
        assert_ok!(invoke(&window, "stop_arrivals", json!({"id":"test"})));
        assert_eq!(streams.response("test").status(), StatusCode::NOT_FOUND);
        let body = json!({"id":"test", "bounds":[0,0,1,1]});
        assert_err!(invoke(&window, "update_arrival_viewport", body));
        let body = json!({"bounds":[0,0,1,1], "channel":"__CHANNEL__:42"});
        let id = assert_ok!(invoke(&window, "start_arrivals", body));
        let id = assert_some!(id.as_str());
        assert!(streams.0.lock().unwrap().contains_key(id));
        assert_ok!(invoke(&window, "stop_arrivals", json!({"id":id})));
        driver.terminate().await;
    }

    #[tokio::test]
    async fn notification_resource_and_stop_share_one_subscription() {
        let driver = spawn(
            SettingsSnapshot::default(),
            Box::new(|_, _, _| unreachable!()),
            Box::new(|_| {}),
            Duration::from_secs(60),
        );
        let streams = ArrivalStreams::default();
        let (sender, mut updates) = tokio::sync::mpsc::unbounded_channel();
        let channel = Channel::new(move |body| {
            sender.send(body.deserialize::<Value>().unwrap()).unwrap();
            Ok(())
        });
        let bounds = assert_ok!(viewport([0., 0., 1., 1.]));
        let id = streams.start(driver.handle.clone(), bounds, channel);
        let update = assert_ok!(tokio::time::timeout(Duration::from_secs(5), updates.recv()).await);
        assert_eq!(
            assert_some!(update),
            json!({"type":"ready", "generation":0, "revision":1})
        );
        let response = streams.response(&id);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/geo+json");
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body: Value = assert_ok!(serde_json::from_slice(response.body()));
        assert_eq!(body, json!({"type":"FeatureCollection", "features":[]}));
        streams.0.lock().unwrap().remove(&id);
        assert_eq!(streams.response(&id).status(), StatusCode::NOT_FOUND);
        driver.terminate().await;
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn worker_failure_notifies_the_subscriber_and_removes_the_resource() {
        let streams = ArrivalStreams::default();
        let (sender, mut updates) = tokio::sync::mpsc::unbounded_channel();
        let channel = Channel::new(move |body| {
            sender.send(body.deserialize::<Value>().unwrap()).unwrap();
            Ok(())
        });
        let bounds = assert_ok!(viewport([0., 0., 1., 1.]));
        let id = streams.start(DriverHandle::stopped(), bounds, channel);
        let update = assert_ok!(tokio::time::timeout(Duration::from_secs(5), updates.recv()).await);
        assert_eq!(assert_some!(update), json!({"type":"failed"}));
        assert_eq!(streams.response(&id).status(), StatusCode::NOT_FOUND);
        assert!(logs_contain("Arrival worker stopped unexpectedly"));
    }
}

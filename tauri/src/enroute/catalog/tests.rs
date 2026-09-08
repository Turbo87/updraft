use super::*;
use claims::{assert_err, assert_ok};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const GERMANY: &str = r#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
const FRANCE: &str = r#"{"maps":[{"path":"Europe/France.mbtiles","size":20,"time":"20260908"}]}"#;

async fn server(body: impl Into<String>, status: u16) -> (String, tokio::task::JoinHandle<()>) {
    let body = body.into();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/maps.json", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(stream.read_u8().await.unwrap());
        }
        assert!(request.starts_with(b"GET /maps.json HTTP/1.1\r\n"));
        let response = format!(
            "HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    (url, task)
}

#[test]
#[tracing_test::traced_test]
fn loads_cache_and_distinguishes_missing_from_invalid_cache() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let missing = CatalogService::load(path.clone()).status();
    assert!(missing.cached.is_none());
    assert!(!missing.error);
    fs::write(&path, GERMANY).unwrap();
    let cached = CatalogService::load(path.clone()).status();
    let cached = cached.cached.unwrap();
    assert_eq!(cached.entries[0].country_code, "DE");
    assert_eq!(
        cached.checked_at,
        fs::metadata(&path).unwrap().modified().unwrap()
    );
    fs::write(&path, b"invalid").unwrap();
    let invalid = CatalogService::load(path).status();
    assert!(invalid.cached.is_none());
    assert!(invalid.error);
    assert!(logs_contain("Could not read Enroute catalog cache"));
}

#[tokio::test]
#[tracing_test::traced_test]
async fn failed_refresh_retains_cache_and_retry_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    fs::write(&path, GERMANY).unwrap();
    let mut service = CatalogService::load(path.clone());
    let cached = service.status().cached.unwrap();
    for (body, status) in [("unavailable", 503), ("invalid", 200), (FRANCE, 200)] {
        let (url, server) = server(body, status).await;
        service.url = url;
        let result = service.refresh().await;
        server.await.unwrap();
        let state = service.status();
        assert!(!state.refreshing);
        if body == FRANCE {
            assert_ok!(result);
            assert!(!state.error);
            assert_eq!(state.cached.unwrap().entries[0].country_code, "FR");
            assert_eq!(fs::read_to_string(&path).unwrap(), FRANCE);
        } else {
            assert_err!(result);
            assert!(state.error);
            assert!(Arc::ptr_eq(&cached, &state.cached.unwrap()));
            assert_eq!(fs::read_to_string(&path).unwrap(), GERMANY);
        }
    }
    let restarted = CatalogService::load(path).status().cached.unwrap();
    assert_eq!(
        restarted.checked_at,
        service.status().cached.unwrap().checked_at
    );
    assert_eq!(restarted.entries[0].country_code, "FR");
    assert!(logs_contain("Could not refresh Enroute catalog"));
}

#[tokio::test]
#[tracing_test::traced_test]
async fn cache_write_failure_retains_last_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    fs::write(&path, GERMANY).unwrap();
    let mut service = CatalogService::load(path.clone());
    let cached = service.status().cached.unwrap();
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let (url, server) = server(FRANCE, 200).await;
    service.url = url;
    assert_err!(service.refresh().await);
    server.await.unwrap();
    assert!(Arc::ptr_eq(&cached, &service.status().cached.unwrap()));
    assert!(service.status().error);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    assert!(logs_contain("Could not refresh Enroute catalog"));
}

#[tokio::test]
async fn refresh_keeps_cache_readable_and_releases_the_lock_on_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    fs::write(&path, GERMANY).unwrap();
    let mut service = CatalogService::load(path);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    service.url = format!("http://{}/maps.json", listener.local_addr().unwrap());
    let service = Arc::new(service);
    let worker = service.clone();
    let request = tokio::spawn(async move { worker.refresh().await });
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        headers.push(stream.read_u8().await.unwrap());
    }
    let status = service.status();
    assert!(status.refreshing);
    assert_eq!(status.cached.unwrap().entries[0].country_code, "DE");
    let error = assert_err!(service.refresh().await);
    assert_eq!(
        error.to_string(),
        "Enroute catalog refresh is already running"
    );
    request.abort();
    assert!(assert_err!(request.await).is_cancelled());
    assert!(!service.status().refreshing);
    let mut remaining = Vec::new();
    let closed =
        tokio::time::timeout(Duration::from_secs(2), stream.read_to_end(&mut remaining)).await;
    assert_eq!(assert_ok!(assert_ok!(closed)), 0);
    assert!(!service.status().error);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[tracing_test::traced_test]
async fn retry_command_reports_success_and_failure_through_ipc() {
    use serde_json::{Value, json};
    let dir = tempfile::tempdir().unwrap();
    for (body, status) in [(FRANCE, 200), ("unavailable", 503)] {
        let (url, server) = server(body, status).await;
        let mut service = CatalogService::load(dir.path().join("catalog.json"));
        service.url = url;
        let service = Arc::new(service);
        let app = tauri::test::mock_builder()
            .manage(service.clone())
            .invoke_handler(tauri::generate_handler![refresh_enroute_catalog])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let request = tauri::webview::InvokeRequest {
            cmd: "refresh_enroute_catalog".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({})),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        };
        let response = tauri::test::get_ipc_response(&window, request)
            .map(|response| response.deserialize::<Value>().unwrap());
        if status == 200 {
            assert_eq!(response, Ok(Value::Null));
        } else {
            assert_eq!(response, Err(json!("Could not refresh Enroute catalog")));
        }
        assert_eq!(service.status().error, status != 200);
        server.await.unwrap();
    }
}

#[tokio::test]
#[tracing_test::traced_test]
async fn rejects_oversized_cache_and_response() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let body = " ".repeat(MAX_CATALOG_BYTES + 1);
    fs::write(&path, &body).unwrap();
    let invalid = CatalogService::load(path.clone()).status();
    assert!(invalid.error);
    assert!(invalid.cached.is_none());
    fs::write(&path, GERMANY).unwrap();
    let mut service = CatalogService::load(path.clone());
    let (url, server) = server(body, 200).await;
    service.url = url;
    let error = assert_err!(service.refresh().await);
    assert_eq!(error.to_string(), "Enroute catalog exceeds size limit");
    server.await.unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), GERMANY);
    assert!(service.status().error);
    assert!(logs_contain("Could not read Enroute catalog cache"));
    assert!(logs_contain("Could not refresh Enroute catalog"));
}

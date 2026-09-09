use super::*;
use claims::{assert_err, assert_ok, assert_some};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use updraft_core::{TrafficTargetId, TrafficTargetIdType};

const OLD: &[u8] = br#"[{"flarm_id":"ABC123","call_sign":"OLD"}]"#;
const NEW: &[u8] = br#"[{"flarm_id":"ABC123","call_sign":"NEW"}]"#;

fn callsign(database: &FlarmnetDatabase) -> &str {
    let id = TrafficTargetId::new(TrafficTargetIdType::Flarm, 0xABC123);
    &assert_some!(database.lookup(id)).call_sign
}

async fn server(body: Vec<u8>, status: u16) -> (String, tokio::task::JoinHandle<()>) {
    let listener = assert_ok!(TcpListener::bind("127.0.0.1:0").await);
    let url = format!("http://{}/united.json", assert_ok!(listener.local_addr()));
    let task = tokio::spawn(async move {
        let (mut stream, _) = assert_ok!(listener.accept().await);
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(assert_ok!(stream.read_u8().await));
        }
        assert!(request.starts_with(b"GET /united.json HTTP/1.1\r\n"));
        let header = format!(
            "HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        assert_ok!(stream.write_all(header.as_bytes()).await);
        assert_ok!(stream.write_all(&body).await);
    });
    (url, task)
}

#[test]
#[tracing_test::traced_test]
fn loads_saved_database_and_recovers_from_missing_or_invalid_cache() {
    let directory = assert_ok!(tempfile::tempdir());
    let service = FlarmnetService::new(directory.path().join("united.json"));
    assert_eq!(service.load(), FlarmnetDatabase::default());
    assert_ok!(fs::write(&service.path, OLD));
    assert_eq!(callsign(&service.load()), "OLD");
    assert_ok!(fs::write(&service.path, b"invalid"));
    assert_eq!(service.load(), FlarmnetDatabase::default());
    assert!(logs_contain("Could not read FlarmNet cache"));
}

#[tokio::test]
async fn failed_downloads_retain_saved_bytes_and_success_replaces_them() {
    let directory = assert_ok!(tempfile::tempdir());
    let mut service = FlarmnetService::new(directory.path().join("united.json"));
    assert_ok!(fs::write(&service.path, OLD));
    for (body, status) in [("unavailable", 503), ("invalid", 200), ("[]", 200)] {
        let (url, server) = server(body.as_bytes().to_vec(), status).await;
        service.url = url;
        assert_err!(service.refresh().await);
        assert_ok!(server.await);
        assert_eq!(assert_ok!(fs::read(&service.path)), OLD);
    }
    let (url, server) = server(NEW.to_vec(), 200).await;
    service.url = url;
    let database = assert_ok!(service.refresh().await);
    assert_eq!(callsign(&database), "NEW");
    assert_ok!(server.await);
    assert_eq!(assert_ok!(fs::read(&service.path)), NEW);
    assert_eq!(callsign(&service.load()), "NEW");
}

#[tokio::test]
async fn failed_install_retains_the_previous_cache() {
    let directory = assert_ok!(tempfile::tempdir());
    let mut service = FlarmnetService::new(directory.path().join("united.json"));
    assert_ok!(fs::create_dir(&service.path));
    assert_ok!(fs::write(service.path.join("old"), OLD));
    let (url, server) = server(NEW.to_vec(), 200).await;
    service.url = url;
    assert_err!(service.refresh().await);
    assert_ok!(server.await);
    assert_eq!(assert_ok!(fs::read(service.path.join("old"))), OLD);
    assert_eq!(assert_ok!(fs::read_dir(directory.path())).count(), 1);
}

#[test]
#[tracing_test::traced_test]
fn rejects_oversized_cache_without_loading_it() {
    let directory = assert_ok!(tempfile::tempdir());
    let service = FlarmnetService::new(directory.path().join("united.json"));
    let file = assert_ok!(File::create(&service.path));
    assert_ok!(file.set_len(MAX_DATABASE_BYTES as u64 + 1));
    assert_eq!(service.load(), FlarmnetDatabase::default());
    assert!(logs_contain("FlarmNet cache exceeds size limit"));
}

#[test]
fn schedules_retries_and_resets_the_refresh_deadline_after_success() {
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let mut schedule = RefreshSchedule::new(start);
    assert_eq!(schedule.remaining(start), Duration::ZERO);
    let mut now = start;
    for minutes in [1, 5, 15, 30, 60, 60] {
        schedule.complete(false, now);
        let delay = Duration::from_secs(minutes * 60);
        assert_eq!(schedule.remaining(now), delay);
        now += delay;
    }
    schedule.complete(true, now);
    assert_eq!(schedule.remaining(now), Duration::from_secs(3 * 60 * 60));
    assert_eq!(
        schedule.remaining(now + Duration::from_secs(4 * 60 * 60)),
        Duration::ZERO
    );
    let next_refresh = now + Duration::from_secs(3 * 60 * 60);
    schedule.complete(false, next_refresh);
    assert_eq!(schedule.remaining(next_refresh), Duration::from_secs(60));
}

#[tokio::test(start_paused = true)]
async fn resume_checks_the_deadline_without_forcing_an_early_download() {
    let now = SystemTime::now();
    let mut schedule = RefreshSchedule::new(now);
    schedule.complete(true, now);
    let wake = Arc::new(tokio::sync::Notify::new());
    let notified = wake.clone();
    let waiting = tokio::spawn(async move { schedule.wait(&notified).await });
    tokio::task::yield_now().await;
    wake.notify_one();
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    tokio::time::advance(Duration::from_secs(3 * 60 * 60)).await;
    assert_ok!(waiting.await);

    let overdue = RefreshSchedule::new(now - Duration::from_secs(60));
    assert_ok!(tokio::time::timeout(Duration::from_secs(1), overdue.wait(&wake)).await);
}

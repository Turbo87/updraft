use super::*;
use crate::enroute::{CatalogEntry, parse_catalog, storage::installed_files};
use claims::{assert_err, assert_le, assert_ok, assert_some_eq};
use std::fs::{self, FileTimes};
use std::io::Write;
use std::time::{Duration, SystemTime};

fn entry() -> CatalogEntry {
    let json = br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
    assert_ok!(parse_catalog(json)).remove(0)
}

#[test]
fn unfinished_downloads_stay_out_of_inventory_and_preserve_installed_files() {
    let directory = tempfile::tempdir().unwrap();
    let entry = entry();
    let parent = directory.path().join("enroute/Europe");
    assert_ok!(fs::create_dir_all(&parent));
    let destination = parent.join("Germany.mbtiles");
    assert_ok!(fs::write(&destination, b"installed"));
    let modified = assert_ok!(assert_ok!(fs::metadata(&destination)).modified());
    let mut download = assert_ok!(DownloadFile::new(directory.path(), &entry));
    assert_ok!(download.file_mut().write_all(b"partial"));
    assert_eq!(assert_ok!(installed_files(directory.path())).len(), 1);
    assert_eq!(assert_ok!(fs::read(&destination)), b"installed");
    drop(download);
    assert_eq!(assert_ok!(fs::read(&destination)), b"installed");
    assert_eq!(
        assert_ok!(assert_ok!(fs::metadata(&destination)).modified()),
        modified
    );
    assert_eq!(assert_ok!(fs::read_dir(parent)).count(), 1);
}

#[test]
fn startup_cleanup_removes_an_abandoned_download() {
    let directory = tempfile::tempdir().unwrap();
    let mut download = assert_ok!(DownloadFile::new(directory.path(), &entry()));
    assert_ok!(download.file_mut().write_all(b"partial"));
    let (file, path) = assert_ok!(download.temporary.keep());
    drop(file);
    assert_ok!(crate::enroute::storage::remove_partial_downloads(
        directory.path()
    ));
    assert!(!path.exists());
    assert!(assert_ok!(installed_files(directory.path())).is_empty());
}

#[test]
fn installation_replaces_bytes_without_validation_or_changing_activation() {
    let entry = entry();
    for (installed, disabled) in [(false, false), (true, false), (true, true)] {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("enroute/Europe/Germany.mbtiles");
        let mut download = assert_ok!(DownloadFile::new(directory.path(), &entry));
        let marker = destination.with_extension("mbtiles.disabled");
        if installed {
            assert_ok!(fs::write(&destination, b"old"));
        }
        if disabled {
            assert_ok!(fs::write(&marker, b""));
        }
        assert_ok!(download.file_mut().write_all(b"not a database"));
        let times = FileTimes::new().set_modified(SystemTime::UNIX_EPOCH);
        assert_ok!(download.file_mut().set_times(times));
        if installed {
            assert_eq!(assert_ok!(fs::read(&destination)), b"old");
        } else {
            assert!(assert_ok!(installed_files(directory.path())).is_empty());
        }
        let before = SystemTime::now();
        assert_ok!(download.install());
        let modified = assert_ok!(assert_ok!(fs::metadata(&destination)).modified());
        let before = assert_ok!(before.duration_since(SystemTime::UNIX_EPOCH)).as_secs();
        let installed_at = assert_ok!(modified.duration_since(SystemTime::UNIX_EPOCH)).as_secs();
        assert_le!(before, installed_at);
        assert_le!(modified, SystemTime::now());
        assert_eq!(assert_ok!(fs::read(&destination)), b"not a database");
        assert_eq!(marker.exists(), disabled);
    }
}

#[test]
fn installation_failure_discards_the_temporary_file() {
    let directory = tempfile::tempdir().unwrap();
    let mut download = assert_ok!(DownloadFile::new(directory.path(), &entry()));
    assert_ok!(download.file_mut().write_all(b"complete"));
    let parent = directory.path().join("enroute/Europe");
    let destination = parent.join("Germany.mbtiles");
    assert_ok!(fs::create_dir(&destination));
    assert_err!(download.install());
    assert!(destination.is_dir());
    assert_eq!(assert_ok!(fs::read_dir(parent)).count(), 1);
}

#[test]
fn update_detection_compares_publication_midnight_with_local_modification_time() {
    let entry = entry();
    let publication: SystemTime = time::macros::datetime!(2026-09-08 0:00 UTC).into();
    assert!(entry.update_available(publication - Duration::from_nanos(1)));
    assert!(!entry.update_available(publication));
    assert!(!entry.update_available(publication + Duration::from_nanos(1)));
}

async fn response(raw: &'static str) -> (reqwest::Response, tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/basemap", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(stream.read_u8().await.unwrap());
        }
        stream.write_all(raw.as_bytes()).await.unwrap();
        stream
    });
    let client = assert_ok!(crate::http::client().build());
    let response = assert_ok!(client.get(url).send().await);
    (response, server.await.unwrap())
}

#[tokio::test]
async fn streams_chunked_bytes_without_installing_or_using_the_catalog_size() {
    let directory = tempfile::tempdir().unwrap();
    let download = assert_ok!(DownloadFile::new(directory.path(), &entry()));
    let destination = directory.path().join("enroute/Europe/Germany.mbtiles");
    assert_ok!(fs::write(&destination, b"installed"));
    let raw =
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nnew \r\n5\r\nbytes\r\n0\r\n\r\n";
    let (response, _connection) = response(raw).await;
    let download = assert_ok!(download.receive(response, |_| {}).await);
    assert_eq!(
        assert_ok!(fs::read(download.temporary.path())),
        b"new bytes"
    );
    assert_eq!(assert_ok!(fs::read(&destination)), b"installed");
    assert_ok!(download.install());
    assert_eq!(assert_ok!(fs::read(destination)), b"new bytes");
}

#[tokio::test]
async fn transfer_failures_discard_partial_files_and_preserve_the_installed_version() {
    for raw in [
        "HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\n\r\n",
        "HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\n\r\nnew",
        "HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nshort",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let download = assert_ok!(DownloadFile::new(directory.path(), &entry()));
        let temporary = download.temporary.path().to_owned();
        let destination = directory.path().join("enroute/Europe/Germany.mbtiles");
        assert_ok!(fs::write(&destination, b"installed"));
        let (response, connection) = response(raw).await;
        drop(connection);
        assert_err!(download.receive(response, |_| {}).await);
        assert!(!temporary.exists());
        assert_eq!(assert_ok!(fs::read(destination)), b"installed");
    }
}

#[tokio::test]
async fn disk_write_failure_discards_the_download() {
    let directory = tempfile::tempdir().unwrap();
    let mut download = assert_ok!(DownloadFile::new(directory.path(), &entry()));
    let temporary = download.temporary.path().to_owned();
    let destination = directory.path().join("enroute/Europe/Germany.mbtiles");
    assert_ok!(fs::write(&destination, b"installed"));
    *download.file_mut() = assert_ok!(fs::File::open(&temporary));
    let (response, _connection) = response("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nnew").await;
    let mut progress = Vec::new();
    let transfer = download.receive(response, |bytes| progress.push(bytes));
    assert_err!(transfer.await);
    assert_eq!(progress, Vec::<u64>::new());
    assert!(!temporary.exists());
    assert_eq!(assert_ok!(fs::read(destination)), b"installed");
}

#[tokio::test]
async fn cancellation_discards_written_bytes_without_installing() {
    let directory = tempfile::tempdir().unwrap();
    let download = assert_ok!(DownloadFile::new(directory.path(), &entry()));
    let temporary = download.temporary.path().to_owned();
    let destination = directory.path().join("enroute/Europe/Germany.mbtiles");
    assert_ok!(fs::write(&destination, b"installed"));
    let (response, _connection) =
        response("HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\npartial").await;
    let worker = tokio::spawn(download.receive(response, |_| {}));
    assert_ok!(
        tokio::time::timeout(Duration::from_secs(2), async {
            while fs::metadata(&temporary).unwrap().len() != 7 {
                tokio::task::yield_now().await;
            }
        })
        .await
    );
    worker.abort();
    assert!(assert_err!(worker.await).is_cancelled());
    assert_ok!(
        tokio::time::timeout(Duration::from_secs(2), async {
            while temporary.exists() {
                tokio::task::yield_now().await;
            }
        })
        .await
    );
    assert_eq!(assert_ok!(fs::read(destination)), b"installed");
}

#[tokio::test]
async fn progress_counts_written_bytes_before_the_transfer_finishes() {
    use tokio::io::AsyncWriteExt;

    let directory = tempfile::tempdir().unwrap();
    let download = assert_ok!(DownloadFile::new(directory.path(), &entry()));
    let temporary = download.temporary.path().to_owned();
    let (response, mut connection) =
        response("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nnew\r\n").await;
    let (sender, mut progress) = tokio::sync::mpsc::unbounded_channel();
    let worker = tokio::spawn(download.receive(response, move |bytes| {
        assert_eq!(assert_ok!(fs::metadata(&temporary)).len(), bytes);
        assert_ok!(sender.send(bytes));
    }));
    let first = tokio::time::timeout(Duration::from_secs(2), progress.recv()).await;
    assert_some_eq!(assert_ok!(first), 3);
    assert!(!worker.is_finished());
    assert_ok!(connection.write_all(b"4\r\n map\r\n0\r\n\r\n").await);
    let completed = tokio::time::timeout(Duration::from_secs(2), worker).await;
    let download = assert_ok!(assert_ok!(assert_ok!(completed)));
    assert_some_eq!(progress.recv().await, 7);
    assert_eq!(progress.recv().await, None);
    assert_eq!(assert_ok!(fs::read(download.temporary.path())), b"new map");
}

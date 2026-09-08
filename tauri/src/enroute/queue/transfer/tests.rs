use super::*;
use crate::enroute::queue::tests::entry;
use claims::{assert_none, assert_ok, assert_some};

#[tokio::test]
#[tracing_test::traced_test]
async fn failures_advance_to_the_next_transfer_without_installing() {
    let directory = tempfile::tempdir().unwrap();
    let queue = Mutex::new(DownloadQueue::default());
    for path in ["Europe/Germany.mbtiles", "Europe/France.mbtiles"] {
        queue.lock().unwrap().enqueue(entry(path));
    }
    let completed = transfer_next_with(&queue, |entry| {
        let directory = directory.path();
        async move {
            if entry.path == "Europe/Germany.mbtiles" {
                anyhow::bail!("test transfer failure");
            }
            DownloadFile::new(directory, &entry)
        }
    })
    .await;
    let (entry, _download) = assert_some!(completed);
    assert_eq!(entry.path, "Europe/France.mbtiles");
    let mut queue = queue.lock().unwrap();
    assert_none!(queue.start_next());
    assert!(queue.failed.contains("Europe/Germany.mbtiles"));
    assert!(queue.finish(&entry, DownloadOutcome::Installed));
    assert!(logs_contain("Could not download Enroute basemap"));
    assert!(logs_contain("test transfer failure"));
    assert!(assert_ok!(crate::enroute::storage::installed_files(directory.path())).is_empty());
}

#[tokio::test]
async fn cancellation_drops_the_transfer_and_advances_to_the_next_file() {
    let directory = tempfile::tempdir().unwrap();
    let queue = Mutex::new(DownloadQueue::default());
    for path in ["Europe/Germany.mbtiles", "Europe/France.mbtiles"] {
        queue.lock().unwrap().enqueue(entry(path));
    }
    let mut status = queue.lock().unwrap().subscribe();
    let transfer = transfer_next_with(&queue, |entry| {
        let directory = directory.path();
        let queue = &queue;
        async move {
            let download = DownloadFile::new(directory, &entry)?;
            if entry.path == "Europe/Germany.mbtiles" {
                queue.lock().unwrap().report_progress(&entry, 3);
                std::future::pending::<()>().await;
            }
            Ok(download)
        }
    });
    let cancel = async {
        loop {
            assert_ok!(status.changed().await);
            if matches!(
                status.borrow()[0].state,
                DownloadState::Downloading { downloaded: 3, .. }
            ) {
                break;
            }
        }
        assert!(queue.lock().unwrap().cancel("Europe/Germany.mbtiles"));
    };
    let completed = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        tokio::join!(transfer, cancel).0
    })
    .await;
    let (entry, download) = assert_some!(assert_ok!(completed));
    assert_eq!(entry.path, "Europe/France.mbtiles");
    drop(download);
    let parent = directory.path().join("enroute/Europe");
    assert_eq!(assert_ok!(std::fs::read_dir(parent)).count(), 0);
}

#[tokio::test]
async fn dropping_the_executor_releases_the_active_attempt() {
    let queue = Mutex::new(DownloadQueue::default());
    let first = entry("Europe/Germany.mbtiles");
    queue.lock().unwrap().enqueue(first);
    let transfer = transfer_next_with(&queue, |_| std::future::pending());
    let mut transfer = Box::pin(transfer);
    let poll = std::future::poll_fn(|cx| std::task::Poll::Ready(transfer.as_mut().poll(cx)));
    assert!(poll.await.is_pending());
    drop(transfer);
    assert!(queue.lock().unwrap().subscribe().borrow().is_empty());
}

#[tokio::test]
async fn cancellation_at_completion_discards_the_result() {
    let directory = tempfile::tempdir().unwrap();
    let queue = Mutex::new(DownloadQueue::default());
    let first = entry("Europe/Germany.mbtiles");
    queue.lock().unwrap().enqueue(first);
    let completed = transfer_next_with(&queue, |entry| {
        let queue = &queue;
        let directory = directory.path();
        async move {
            let download = DownloadFile::new(directory, &entry)?;
            queue.lock().unwrap().cancel(entry.path);
            Ok(download)
        }
    })
    .await;
    assert_none!(completed);
    let parent = directory.path().join("enroute/Europe");
    assert_eq!(assert_ok!(std::fs::read_dir(parent)).count(), 0);
}

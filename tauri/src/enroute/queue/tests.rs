use super::*;
use claims::{assert_none, assert_some};
use time::macros::date;

fn entry(path: &'static str) -> BasemapEntry {
    BasemapEntry {
        path,
        country_code: "DE",
        continent: super::super::Continent::Europe,
        size: 10.try_into().unwrap(),
        publication_date: date!(2026 - 09 - 08),
    }
}

#[test]
fn starts_one_download_at_a_time_in_submission_order() {
    let mut queue = DownloadQueue::default();
    assert_none!(queue.start_next());
    assert!(queue.enqueue(entry("Europe/Germany.mbtiles")));
    assert!(queue.enqueue(entry("Europe/France.mbtiles")));
    assert!(!queue.enqueue(entry("Europe/Germany.mbtiles")));
    let first = assert_some!(queue.start_next());
    assert_eq!(first.path, "Europe/Germany.mbtiles");
    assert_none!(queue.start_next());
    assert!(!queue.enqueue(entry(first.path)));
    assert!(queue.enqueue(entry("Europe/Italy.mbtiles")));
    assert!(queue.finish(&first, DownloadOutcome::Installed));
    let second = assert_some!(queue.start_next());
    assert_eq!(second.path, "Europe/France.mbtiles");
    assert!(queue.finish(&second, DownloadOutcome::Installed));
    let third = assert_some!(queue.start_next());
    assert_eq!(third.path, "Europe/Italy.mbtiles");
    assert!(queue.finish(&third, DownloadOutcome::Installed));
    assert_none!(queue.start_next());
    assert!(queue.failed.is_empty());
}

#[test]
fn failure_advances_the_queue_and_retry_joins_the_tail() {
    let mut queue = DownloadQueue::default();
    queue.enqueue(entry("Europe/Germany.mbtiles"));
    queue.enqueue(entry("Europe/France.mbtiles"));
    let first = assert_some!(queue.start_next());
    assert!(queue.finish(&first, DownloadOutcome::Failed));
    assert!(queue.failed.contains(first.path));
    assert!(!queue.cancel(first.path));
    assert!(queue.enqueue(entry(first.path)));
    assert!(queue.failed.contains(first.path));
    let second = assert_some!(queue.start_next());
    assert_eq!(second.path, "Europe/France.mbtiles");
    assert!(queue.finish(&second, DownloadOutcome::Installed));
    let retry = assert_some!(queue.start_next());
    assert_eq!(retry.path, first.path);
    assert!(queue.failed.contains(first.path));
    assert!(queue.finish(&retry, DownloadOutcome::Installed));
    assert!(queue.failed.is_empty());
}

#[test]
fn cancellation_removes_pending_and_active_attempts() {
    let mut queue = DownloadQueue::default();
    queue.enqueue(entry("Europe/Germany.mbtiles"));
    queue.enqueue(entry("Europe/France.mbtiles"));
    queue.enqueue(entry("Europe/Italy.mbtiles"));
    let first = assert_some!(queue.start_next());
    assert!(!queue.cancel("unknown"));
    assert!(queue.cancel("Europe/France.mbtiles"));
    assert!(queue.cancel(first.path));
    assert!(!queue.cancel(first.path));
    let next = assert_some!(queue.start_next());
    assert_eq!(next.path, "Europe/Italy.mbtiles");
    assert!(!queue.finish(&first, DownloadOutcome::Failed));
    assert!(queue.failed.is_empty());
    assert!(queue.finish(&next, DownloadOutcome::Installed));
    assert_none!(queue.start_next());
}

#[test]
fn cancelled_retries_retain_failure_and_ignore_late_completion() {
    let mut queue = DownloadQueue::default();
    queue.enqueue(entry("Europe/Germany.mbtiles"));
    let first = assert_some!(queue.start_next());
    queue.finish(&first, DownloadOutcome::Failed);
    let mut cancelled = Arc::clone(&first);
    for active in [false, true] {
        assert!(queue.enqueue(entry(first.path)));
        if active {
            cancelled = assert_some!(queue.start_next());
        }
        assert!(queue.cancel(first.path));
        assert!(queue.failed.contains(first.path));
    }
    assert!(queue.enqueue(entry(first.path)));
    let retry = assert_some!(queue.start_next());
    assert!(!queue.finish(&cancelled, DownloadOutcome::Installed));
    assert!(queue.failed.contains(first.path));
    assert_none!(queue.start_next());
    assert!(queue.finish(&retry, DownloadOutcome::Installed));
    assert!(!queue.finish(&retry, DownloadOutcome::Failed));
    assert!(queue.failed.is_empty());
}

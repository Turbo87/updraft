use super::CatalogEntry;
use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::Instant;

mod transfer;

#[derive(Debug)]
pub struct DownloadQueue {
    pending: VecDeque<Arc<CatalogEntry>>,
    active: Option<ActiveDownload>,
    failed: BTreeSet<&'static str>,
    status: watch::Sender<Vec<DownloadStatus>>,
}

#[derive(Debug)]
struct ActiveDownload {
    entry: Arc<CatalogEntry>,
    downloaded: u64,
    progress_published: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DownloadStatus {
    pub path: &'static str,
    #[serde(flatten)]
    pub state: DownloadState,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DownloadState {
    Queued,
    Downloading { downloaded: u64, total: u64 },
    Failed,
}

impl Default for DownloadQueue {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
            active: None,
            failed: BTreeSet::new(),
            status: watch::channel(Vec::new()).0,
        }
    }
}

#[derive(Debug)]
pub enum DownloadOutcome {
    Installed,
    Failed,
}

impl DownloadQueue {
    /// Subscribers receive the current state and the latest state after changes.
    pub fn subscribe(&self) -> watch::Receiver<Vec<DownloadStatus>> {
        self.status.subscribe()
    }

    /// Ignores stale attempts and non-increasing byte counts.
    /// Publishes byte progress at most every 100 ms to avoid flooding the webview.
    /// Queue transitions still publish the latest byte count immediately.
    pub fn report_progress(&mut self, attempt: &Arc<CatalogEntry>, downloaded: u64) -> bool {
        let Some(active) = &mut self.active else {
            return false;
        };
        if !Arc::ptr_eq(&active.entry, attempt) || downloaded <= active.downloaded {
            return false;
        }
        active.downloaded = downloaded;
        let now = Instant::now();
        let interval = Duration::from_millis(100);
        if active
            .progress_published
            .is_none_or(|last| now - last >= interval)
        {
            active.progress_published = Some(now);
            self.publish();
        }
        true
    }

    fn publish(&self) {
        let mut status = Vec::new();
        if let Some(active) = &self.active {
            status.push(DownloadStatus {
                path: active.entry.path,
                state: DownloadState::Downloading {
                    downloaded: active.downloaded,
                    total: active.entry.size.get(),
                },
            });
        }
        status.extend(self.pending.iter().map(|entry| DownloadStatus {
            path: entry.path,
            state: DownloadState::Queued,
        }));
        for &path in &self.failed {
            if !status.iter().any(|item| item.path == path) {
                status.push(DownloadStatus {
                    path,
                    state: DownloadState::Failed,
                });
            }
        }
        self.status.send_replace(status);
    }

    /// Adds a new attempt at the tail. Queued and active paths cannot be added again.
    pub fn enqueue(&mut self, entry: CatalogEntry) -> bool {
        let active = self.active.iter().map(|active| &active.entry);
        let mut attempts = self.pending.iter().chain(active);
        if attempts.any(|queued| queued.path == entry.path) {
            return false;
        }
        self.pending.push_back(Arc::new(entry));
        self.publish();
        true
    }

    pub fn has_active(&self) -> bool {
        self.active.is_some()
    }

    /// Returns an attempt only when no download is active.
    pub fn start_next(&mut self) -> Option<Arc<CatalogEntry>> {
        if self.active.is_some() {
            return None;
        }
        let attempt = self.pending.pop_front()?;
        self.active = Some(ActiveDownload {
            entry: Arc::clone(&attempt),
            downloaded: 0,
            progress_published: None,
        });
        self.publish();
        Some(attempt)
    }

    /// Returns whether the attempt still owns the active queue slot.
    pub fn is_active(&self, attempt: &Arc<CatalogEntry>) -> bool {
        let active = self.active.as_ref();
        active.is_some_and(|active| Arc::ptr_eq(&active.entry, attempt))
    }

    /// Records installation or failure. Transfer completion alone is not installation.
    /// Only the current attempt from `start_next()` can finish the download.
    pub fn finish(&mut self, attempt: &Arc<CatalogEntry>, outcome: DownloadOutcome) -> bool {
        if !self.is_active(attempt) {
            return false;
        }
        self.active = None;
        match outcome {
            DownloadOutcome::Installed => {
                self.failed.remove(attempt.path);
            }
            DownloadOutcome::Failed => {
                self.failed.insert(attempt.path);
            }
        }
        self.publish();
        true
    }

    /// Cancels queue state. The caller must also stop the transfer before installation.
    /// Previous failures remain recorded when a retry is cancelled.
    pub fn cancel(&mut self, path: &str) -> bool {
        let active = self.active.as_ref();
        if active.is_some_and(|active| active.entry.path == path) {
            self.active = None;
            self.publish();
            return true;
        }
        let Some(index) = self.pending.iter().position(|entry| entry.path == path) else {
            return false;
        };
        self.pending.remove(index);
        self.publish();
        true
    }
}

#[cfg(test)]
mod tests;

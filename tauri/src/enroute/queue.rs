use super::BasemapEntry;
use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Debug)]
pub struct DownloadQueue {
    pending: VecDeque<Arc<BasemapEntry>>,
    active: Option<ActiveDownload>,
    failed: BTreeSet<&'static str>,
    status: watch::Sender<Vec<DownloadStatus>>,
}

#[derive(Debug)]
struct ActiveDownload {
    entry: Arc<BasemapEntry>,
    downloaded: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadStatus {
    pub path: &'static str,
    pub state: DownloadState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub fn report_progress(&mut self, attempt: &Arc<BasemapEntry>, downloaded: u64) -> bool {
        let Some(active) = &mut self.active else {
            return false;
        };
        if !Arc::ptr_eq(&active.entry, attempt) || downloaded <= active.downloaded {
            return false;
        }
        active.downloaded = downloaded;
        self.publish();
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
    pub fn enqueue(&mut self, entry: BasemapEntry) -> bool {
        let active = self.active.iter().map(|active| &active.entry);
        let mut attempts = self.pending.iter().chain(active);
        if attempts.any(|queued| queued.path == entry.path) {
            return false;
        }
        self.pending.push_back(Arc::new(entry));
        self.publish();
        true
    }

    /// Returns an attempt only when no download is active.
    pub fn start_next(&mut self) -> Option<Arc<BasemapEntry>> {
        if self.active.is_some() {
            return None;
        }
        let attempt = self.pending.pop_front()?;
        self.active = Some(ActiveDownload {
            entry: Arc::clone(&attempt),
            downloaded: 0,
        });
        self.publish();
        Some(attempt)
    }

    /// Records installation or failure. Transfer completion alone is not installation.
    /// Only the current attempt from `start_next()` can finish the download.
    pub fn finish(&mut self, attempt: &Arc<BasemapEntry>, outcome: DownloadOutcome) -> bool {
        let active = self.active.as_ref();
        let current = active.is_some_and(|active| Arc::ptr_eq(&active.entry, attempt));
        if !current {
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

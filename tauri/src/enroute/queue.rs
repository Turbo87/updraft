use super::BasemapEntry;
use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct DownloadQueue {
    pending: VecDeque<Arc<BasemapEntry>>,
    active: Option<Arc<BasemapEntry>>,
    failed: BTreeSet<&'static str>,
}

#[derive(Debug)]
pub enum DownloadOutcome {
    Installed,
    Failed,
}

impl DownloadQueue {
    /// Adds a new attempt at the tail. Queued and active paths cannot be added again.
    pub fn enqueue(&mut self, entry: BasemapEntry) -> bool {
        let mut attempts = self.pending.iter().chain(self.active.iter());
        if attempts.any(|queued| queued.path == entry.path) {
            return false;
        }
        self.pending.push_back(Arc::new(entry));
        true
    }

    /// Returns an attempt only when no download is active.
    pub fn start_next(&mut self) -> Option<Arc<BasemapEntry>> {
        if self.active.is_some() {
            return None;
        }
        let attempt = self.pending.pop_front()?;
        self.active = Some(Arc::clone(&attempt));
        Some(attempt)
    }

    /// Records installation or failure. Transfer completion alone is not installation.
    /// Only the current attempt from `start_next()` can finish the download.
    pub fn finish(&mut self, attempt: &Arc<BasemapEntry>, outcome: DownloadOutcome) -> bool {
        let active = self.active.as_ref();
        let current = active.is_some_and(|active| Arc::ptr_eq(active, attempt));
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
        true
    }

    /// Cancels queue state. The caller must also stop the transfer before installation.
    /// Previous failures remain recorded when a retry is cancelled.
    pub fn cancel(&mut self, path: &str) -> bool {
        if self.active.as_ref().is_some_and(|entry| entry.path == path) {
            self.active = None;
            return true;
        }
        let Some(index) = self.pending.iter().position(|entry| entry.path == path) else {
            return false;
        };
        self.pending.remove(index);
        true
    }
}

#[cfg(test)]
mod tests;

use super::*;
use crate::enroute::download::BasemapDownload;
use anyhow::Result;
use std::path::Path;
use std::sync::Mutex;

impl DownloadQueue {
    /// Skips failed or cancelled transfers and returns the next complete temporary file.
    /// The returned attempt stays active until the caller installs it and calls `finish()`.
    /// Callers must recheck `is_active()` under the queue lock before installation.
    pub async fn transfer_next(
        queue: &Mutex<Self>,
        directory: &Path,
    ) -> Option<(Arc<BasemapEntry>, BasemapDownload)> {
        transfer_next_with(queue, |entry| async move {
            BasemapDownload::fetch(directory, &entry, |bytes| {
                queue.lock().unwrap().report_progress(&entry, bytes);
            })
            .await
        })
        .await
    }
}

struct TransferAttempt<'a> {
    queue: &'a Mutex<DownloadQueue>,
    entry: Arc<BasemapEntry>,
    completed: bool,
}

impl Drop for TransferAttempt<'_> {
    fn drop(&mut self) {
        if !self.completed {
            let mut queue = self.queue.lock().unwrap();
            if queue.is_active(&self.entry) {
                queue.cancel(self.entry.path);
            }
        }
    }
}

async fn transfer_next_with<F: Future<Output = Result<BasemapDownload>>>(
    queue: &Mutex<DownloadQueue>,
    mut fetch: impl FnMut(Arc<BasemapEntry>) -> F,
) -> Option<(Arc<BasemapEntry>, BasemapDownload)> {
    loop {
        let (entry, mut status) = {
            let mut queue = queue.lock().unwrap();
            (queue.start_next()?, queue.subscribe())
        };
        let mut attempt = TransferAttempt {
            queue,
            entry: Arc::clone(&entry),
            completed: false,
        };
        let cancelled = async {
            while queue.lock().unwrap().is_active(&entry) {
                status
                    .changed()
                    .await
                    .expect("The queue owns the status sender");
            }
        };
        let result = tokio::select! {
            result = fetch(Arc::clone(&entry)) => Some(result),
            () = cancelled => None,
        };
        let mut queue = queue.lock().unwrap();
        if !queue.is_active(&entry) {
            continue;
        }
        let Some(result) = result else {
            continue;
        };
        match result {
            Ok(download) => {
                attempt.completed = true;
                return Some((entry, download));
            }
            Err(error) => {
                tracing::warn!(%error, path = entry.path, "Could not download Enroute basemap");
                queue.finish(&entry, DownloadOutcome::Failed);
            }
        }
    }
}

#[cfg(test)]
mod tests;

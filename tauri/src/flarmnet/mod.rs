use crate::driver::DriverHandle;
use anyhow::{Context, Result, ensure};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;
use tokio::sync::Notify;
use updraft_core::{FlarmnetDatabase, ReplaceFlarmnetDatabase};

const DATABASE_URL: &str = "https://turbo87.github.io/united-flarmnet/united.json";
const MAX_DATABASE_BYTES: usize = 8 * 1024 * 1024;

pub struct FlarmnetService {
    path: PathBuf,
    url: String,
    wake: Notify,
}

impl FlarmnetService {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            url: DATABASE_URL.into(),
            wake: Notify::new(),
        }
    }

    pub fn start(self: Arc<Self>, driver: DriverHandle) -> Result<()> {
        let database = Arc::new(self.load());
        tauri::async_runtime::block_on(driver.send(ReplaceFlarmnetDatabase(database)))?;
        tauri::async_runtime::spawn(async move {
            if let Err(error) = self.run(driver).await {
                tracing::error!(?error, "FlarmNet refresh worker stopped");
            }
        });
        Ok(())
    }

    pub fn check_due(&self) {
        self.wake.notify_one();
    }

    async fn run(&self, driver: DriverHandle) -> Result<()> {
        let mut schedule = RefreshSchedule::new(SystemTime::now());
        loop {
            schedule.wait(&self.wake).await;
            let refreshed = match self.refresh().await {
                Ok(database) => {
                    let input = ReplaceFlarmnetDatabase(Arc::new(database));
                    driver.send(input).await?;
                    true
                }
                Err(error) => {
                    tracing::warn!(?error, "Could not refresh FlarmNet database");
                    false
                }
            };
            schedule.complete(refreshed, SystemTime::now());
        }
    }

    fn load(&self) -> FlarmnetDatabase {
        let cached = (|| -> Result<_> {
            let file = match File::open(&self.path) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error.into()),
            };
            let mut bytes = Vec::new();
            file.take(MAX_DATABASE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= MAX_DATABASE_BYTES,
                "FlarmNet cache exceeds size limit"
            );
            Ok(Some(FlarmnetDatabase::from_json(&bytes)?))
        })();
        match cached {
            Ok(database) => database.unwrap_or_default(),
            Err(error) => {
                tracing::warn!(?error, "Could not read FlarmNet cache");
                FlarmnetDatabase::default()
            }
        }
    }

    async fn refresh(&self) -> Result<FlarmnetDatabase> {
        let timeout = Duration::from_secs(30);
        let client = crate::http::client().timeout(timeout).build()?;
        let mut response = client.get(&self.url).send().await?.error_for_status()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= MAX_DATABASE_BYTES,
                "FlarmNet download exceeds size limit"
            );
            bytes.extend_from_slice(&chunk);
        }
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let database = FlarmnetDatabase::from_json(&bytes)?;
            let directory = path.parent().context("FlarmNet cache path has no parent")?;
            fs::create_dir_all(directory)?;
            let mut file = NamedTempFile::new_in(directory)?;
            file.write_all(&bytes)?;
            file.as_file().sync_all()?;
            file.persist(path).map_err(|error| error.error)?;
            Ok(database)
        })
        .await?
    }
}

struct RefreshSchedule {
    due: SystemTime,
    failures: usize,
}

impl RefreshSchedule {
    fn new(now: SystemTime) -> Self {
        Self {
            due: now,
            failures: 0,
        }
    }

    fn complete(&mut self, success: bool, now: SystemTime) {
        let minutes = if success {
            self.failures = 0;
            180
        } else {
            let retries = [1, 5, 15, 30, 60];
            let minutes = retries[self.failures];
            self.failures = (self.failures + 1).min(retries.len() - 1);
            minutes
        };
        self.due = now + Duration::from_secs(minutes * 60);
    }

    fn remaining(&self, now: SystemTime) -> Duration {
        self.due.duration_since(now).unwrap_or_default()
    }

    async fn wait(&self, wake: &Notify) {
        loop {
            let remaining = self.remaining(SystemTime::now());
            tokio::select! {
                _ = tokio::time::sleep(remaining) => return,
                _ = wake.notified() => {},
            }
        }
    }
}

#[cfg(test)]
mod tests;

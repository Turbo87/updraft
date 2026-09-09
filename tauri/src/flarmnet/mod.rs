use crate::driver::DriverHandle;
use anyhow::{Context, Result, ensure};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use updraft_core::{FlarmnetDatabase, ReplaceFlarmnetDatabase};

const DATABASE_URL: &str = "https://turbo87.github.io/united-flarmnet/united.json";
const MAX_DATABASE_BYTES: usize = 8 * 1024 * 1024;

pub struct FlarmnetService {
    path: PathBuf,
    url: String,
}

impl FlarmnetService {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            url: DATABASE_URL.into(),
        }
    }

    pub fn start(self, driver: DriverHandle) -> Result<()> {
        let database = Arc::new(self.load());
        tauri::async_runtime::block_on(driver.send(ReplaceFlarmnetDatabase(database)))?;
        tauri::async_runtime::spawn(async move {
            match self.refresh().await {
                Ok(database) => {
                    let input = ReplaceFlarmnetDatabase(Arc::new(database));
                    if let Err(error) = driver.send(input).await {
                        tracing::error!(%error, "Could not publish FlarmNet database");
                    }
                }
                Err(error) => tracing::warn!(?error, "Could not refresh FlarmNet database"),
            }
        });
        Ok(())
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

#[cfg(test)]
mod tests;

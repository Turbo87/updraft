use super::{BasemapEntry, parse_catalog};
use anyhow::{Context, Result, ensure};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;
use tokio::sync::{MutexGuard, watch};

pub mod commands;

const MAX_CATALOG_BYTES: usize = 4 * 1024 * 1024;
const CATALOG_URL: &str = "https://enroute-data.akaflieg-freiburg.de/enroute-GeoJSONv003/maps.json";

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedCatalog {
    pub entries: Vec<BasemapEntry>,
    #[serde(serialize_with = "serialize_checked_at")]
    pub checked_at: SystemTime,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CatalogStatus {
    pub cached: Option<Arc<CachedCatalog>>,
    pub refreshing: bool,
    pub error: bool,
}

#[derive(Debug, Default)]
struct CacheState {
    cached: Option<Arc<CachedCatalog>>,
    error: bool,
}

#[derive(Debug)]
pub struct CatalogService {
    path: PathBuf,
    url: String,
    state: Mutex<CacheState>,
    refresh: tokio::sync::Mutex<()>,
    updates: watch::Sender<CatalogStatus>,
}

struct CatalogRefresh<'a> {
    service: &'a CatalogService,
    lock: Option<MutexGuard<'a, ()>>,
}

impl Drop for CatalogRefresh<'_> {
    fn drop(&mut self) {
        drop(self.lock.take());
        self.service.publish();
    }
}

impl CatalogService {
    /// Loads the last successful catalog. Cache errors do not prevent a refresh.
    pub fn load(path: PathBuf) -> Self {
        let state = match read_cache(&path) {
            Ok(cached) => CacheState {
                cached: cached.map(Arc::new),
                error: false,
            },
            Err(error) => {
                tracing::warn!(?error, "Could not read Enroute catalog cache");
                CacheState {
                    cached: None,
                    error: true,
                }
            }
        };
        let initial = CatalogStatus {
            cached: state.cached.clone(),
            refreshing: false,
            error: state.error,
        };
        Self {
            path,
            url: CATALOG_URL.into(),
            state: Mutex::new(state),
            refresh: tokio::sync::Mutex::new(()),
            updates: watch::channel(initial).0,
        }
    }

    pub fn status(&self) -> CatalogStatus {
        let state = self.state.lock().expect("Catalog access should not panic");
        let refreshing = self.refresh.try_lock().is_err();
        CatalogStatus {
            cached: state.cached.clone(),
            refreshing,
            error: state.error,
        }
    }

    /// Provides the current cached catalog and subsequent refresh status changes.
    pub fn subscribe(&self) -> watch::Receiver<CatalogStatus> {
        self.updates.subscribe()
    }

    fn publish(&self) {
        // Read inside the update lock so an older snapshot cannot overwrite a newer one.
        self.updates.send_modify(|status| *status = self.status());
    }

    /// Refreshes and atomically caches the catalog. An overlapping call fails.
    /// Failures retain the previous catalog and its last-success timestamp.
    pub async fn refresh(&self) -> Result<()> {
        let lock = self
            .refresh
            .try_lock()
            .context("Enroute catalog refresh is already running")?;
        let _refresh = CatalogRefresh {
            service: self,
            lock: Some(lock),
        };
        self.publish();
        let result = self.fetch().await;
        let mut state = self.state.lock().expect("Catalog access should not panic");
        match result {
            Ok(cached) => {
                state.cached = Some(Arc::new(cached));
                state.error = false;
                Ok(())
            }
            Err(error) => {
                state.error = true;
                tracing::warn!(?error, "Could not refresh Enroute catalog");
                Err(error)
            }
        }
    }

    async fn fetch(&self) -> Result<CachedCatalog> {
        let client = super::http_client()
            .timeout(Duration::from_secs(30))
            .build()?;
        let mut response = client.get(&self.url).send().await?.error_for_status()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= MAX_CATALOG_BYTES,
                "Enroute catalog exceeds size limit"
            );
            bytes.extend_from_slice(&chunk);
        }
        let entries = parse_catalog(&bytes)?;
        let directory = self
            .path
            .parent()
            .context("Catalog cache path has no parent")?;
        fs::create_dir_all(directory)?;
        let mut temporary = NamedTempFile::new_in(directory)?;
        temporary.write_all(&bytes)?;
        let checked_at = temporary.as_file().metadata()?.modified()?;
        temporary.persist(&self.path).map_err(|error| error.error)?;
        Ok(CachedCatalog {
            entries,
            checked_at,
        })
    }
}

fn read_cache(path: &Path) -> Result<Option<CachedCatalog>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_CATALOG_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_CATALOG_BYTES,
        "Enroute catalog cache exceeds size limit"
    );
    Ok(Some(CachedCatalog {
        entries: parse_catalog(&bytes)?,
        checked_at: file.metadata()?.modified()?,
    }))
}

#[tauri::command]
pub async fn refresh_enroute_catalog(
    state: tauri::State<'_, Arc<CatalogService>>,
) -> Result<(), &'static str> {
    state
        .refresh()
        .await
        .map_err(|_| "Could not refresh Enroute catalog")
}

fn serialize_checked_at<S: serde::Serializer>(
    time: &SystemTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let seconds = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64(),
        Err(error) => -error.duration().as_secs_f64(),
    };
    serializer.serialize_f64(seconds * 1000.)
}

#[cfg(test)]
mod tests;

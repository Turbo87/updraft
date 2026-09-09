use self::commands::TerrainStatus;
use crate::enroute::{CatalogEntry, download::DownloadFile};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tauri::http::{Response, StatusCode, header};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use updraft_terrain::TerrainReader;

pub mod commands;
mod worker;
pub use worker::watch_elevation;

pub struct Terrain {
    files: BTreeMap<String, TerrainSource>,
    directory: PathBuf,
    generation: u64,
    changes: tokio::sync::watch::Sender<u64>,
    reader: TerrainReader,
    subscribers: BTreeMap<u32, Channel<TerrainStatus>>,
}

#[derive(Debug)]
enum TerrainSource {
    Active,
    Disabled,
    Unavailable(anyhow::Error),
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            files: BTreeMap::new(),
            directory: PathBuf::new(),
            generation: 0,
            changes: tokio::sync::watch::channel(0).0,
            subscribers: BTreeMap::new(),
            reader: TerrainReader::default(),
        }
    }
}

impl Terrain {
    pub fn load(directory: &Path) -> Result<Self> {
        let mut files = BTreeMap::new();
        let mut reader = TerrainReader::default();
        for (id, path) in crate::enroute::storage::installed_files(directory)? {
            if !id.ends_with(".terrain") {
                continue;
            }
            let source = if path.with_extension("terrain.disabled").try_exists()? {
                TerrainSource::Disabled
            } else {
                match reader.insert(id.clone(), &path) {
                    Ok(()) => TerrainSource::Active,
                    Err(error) => TerrainSource::Unavailable(error),
                }
            };
            if let TerrainSource::Unavailable(error) = &source {
                tracing::warn!(%error, path = %path.display(), "Could not open offline terrain");
            }
            files.insert(id, source);
        }
        Ok(Self {
            files,
            reader,
            directory: directory.to_owned(),
            ..Self::default()
        })
    }

    pub fn available_updates(&self, entries: &[CatalogEntry]) -> Result<Vec<&'static str>> {
        let mut updates = Vec::new();
        for entry in entries {
            let name = format!("enroute/{}", entry.path);
            if !self.files.contains_key(&name) {
                continue;
            }
            let modified = fs::metadata(self.directory.join(&name))
                .and_then(|metadata| metadata.modified())
                .with_context(|| format!("Could not read terrain timestamp for {name}"))?;
            if entry.update_available(modified) {
                updates.push(entry.path);
            }
        }
        Ok(updates)
    }

    pub fn resource_response(&self, path: &str) -> Response<Vec<u8>> {
        let Some((generation, path)) = path
            .split_once('/')
            .and_then(|(generation, path)| Some((generation.parse::<u64>().ok()?, path)))
        else {
            return error_response(StatusCode::BAD_REQUEST);
        };
        if generation != self.generation {
            return error_response(StatusCode::NOT_FOUND);
        }
        let (content_type, result) = if path == "metadata.json" {
            ("application/json", self.metadata().map(Some))
        } else if let Some([z, x, y]) = terrain_coordinates(path) {
            ("image/webp", self.reader.tile(z, x, y))
        } else {
            return error_response(StatusCode::BAD_REQUEST);
        };
        let (status, body) = match result {
            Ok(Some(data)) => (StatusCode::OK, data),
            Ok(None) => (StatusCode::NOT_FOUND, Vec::new()),
            Err(error) => {
                tracing::warn!(?error, path, "Could not read offline terrain resource");
                (StatusCode::INTERNAL_SERVER_ERROR, Vec::new())
            }
        };
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "no-store")
            .body(body)
            .expect("the fixed terrain response should be valid")
    }

    fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        ensure!(
            self.files.contains_key(name),
            "Terrain file is not installed"
        );
        let path = self.directory.join(name);
        let marker = path.with_extension("terrain.disabled");
        if enabled {
            match fs::remove_file(&marker) {
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        } else {
            match fs::File::create_new(&marker) {
                Ok(_) => {}
                Err(error)
                    if error.kind() == ErrorKind::AlreadyExists
                        && fs::symlink_metadata(&marker)?.is_file() => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.recheck(|id, source| {
            if id == name {
                enabled
            } else {
                !matches!(source, TerrainSource::Disabled)
            }
        });
        Ok(())
    }

    pub fn install_download(&mut self, name: &str, download: DownloadFile) -> Result<()> {
        let path = self.directory.join(name);
        ensure!(
            download.destination() == path,
            "Terrain download destination does not match"
        );
        if !self.files.contains_key(name) {
            let marker = path.with_extension("terrain.disabled");
            match fs::remove_file(marker) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let previous = self.files.remove(name);
        let installed = previous.is_some();
        let enabled = !matches!(previous, Some(TerrainSource::Disabled));
        // Close SQLite before replacement, including on Windows.
        self.reader.remove(name);
        let result = download.install();
        if result.is_ok() || installed {
            self.files.insert(name.to_owned(), TerrainSource::Disabled);
            self.recheck(|id, source| {
                if id == name {
                    enabled
                } else {
                    !matches!(source, TerrainSource::Disabled)
                }
            });
        }
        result
    }

    fn remove(&mut self, name: &str) -> Result<()> {
        let path = self.directory.join(name);
        let source = self
            .files
            .get_mut(name)
            .context("Terrain file is not installed")?;
        let enabled = !matches!(source, TerrainSource::Disabled);
        // Close SQLite before deletion, including on Windows.
        *source = TerrainSource::Disabled;
        self.reader.remove(name);
        let marker = path.with_extension("terrain.disabled");
        let paths = [&path, &marker];
        let result = paths
            .into_iter()
            .try_for_each(|path| match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            });
        if result.is_ok() {
            self.files.remove(name);
        }
        self.recheck(|id, source| {
            if id == name {
                enabled
            } else {
                !matches!(source, TerrainSource::Disabled)
            }
        });
        result.map_err(Into::into)
    }

    fn recheck(&mut self, is_enabled: impl Fn(&str, &TerrainSource) -> bool) {
        self.reader.clear();
        for (id, source) in &mut self.files {
            if !is_enabled(id, source) {
                *source = TerrainSource::Disabled;
                continue;
            }
            let path = self.directory.join(id);
            *source = match self.reader.insert(id.clone(), &path) {
                Ok(()) => TerrainSource::Active,
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "Could not open offline terrain");
                    TerrainSource::Unavailable(error)
                }
            };
        }
        self.generation += 1;
        self.changes.send_replace(self.generation);
        self.publish();
    }

    fn metadata(&self) -> Result<Vec<u8>> {
        let terrain = self.reader.metadata();
        let generation = self.generation;
        let tiles = format!("updraft://localhost/terrain/{generation}/{{z}}/{{x}}/{{y}}.webp");
        let mut metadata = serde_json::json!({
            "tilejson": "3.0.0",
            "tiles": [tiles],
            "encoding": "terrarium",
            "attribution": terrain.attributions.join("<br>"),
        });
        if let Some(coverage) = terrain.coverage {
            metadata["tileSize"] = coverage.tile_size.into();
            metadata["minzoom"] = coverage.min_zoom.into();
            metadata["maxzoom"] = coverage.max_zoom.into();
        }
        Ok(serde_json::to_vec(&metadata)?)
    }
}

/// Reads terrain tiles and metadata on a blocking worker.
pub async fn terrain_resource_response<R: tauri::Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Response<Vec<u8>> {
    let terrain = app.state::<Arc<Mutex<Terrain>>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        terrain
            .lock()
            .expect("terrain access should not panic")
            .resource_response(&path)
    })
    .await
    .unwrap_or_else(|error| {
        tracing::warn!(%error, "Offline terrain resource worker failed");
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::new())
            .expect("the fixed error response should be valid")
    })
}

fn error_response(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CACHE_CONTROL, "no-store")
        .body(Vec::new())
        .expect("the fixed error response should be valid")
}

fn terrain_coordinates(path: &str) -> Option<[u32; 3]> {
    let mut parts = path.strip_suffix(".webp")?.split('/');
    let z = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let size = 1_u32.checked_shl(z)?;
    (parts.next().is_none() && x < size && y < size).then_some([z, x, y])
}

#[cfg(test)]
mod tests;

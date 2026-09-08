use self::commands::BasemapStatus;
use crate::enroute::download::BasemapDownload;
use anyhow::{Context, Result, ensure};
use flate2::read::GzDecoder;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::http::{Response, StatusCode, header};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

pub mod commands;

const TILE_QUERY: &str =
    "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3";

#[derive(Default)]
pub struct Basemaps {
    files: BTreeMap<String, BasemapSource>,
    directory: PathBuf,
    generation: u64,
    subscribers: BTreeMap<u32, Channel<BasemapStatus>>,
}

#[derive(Debug)]
enum BasemapSource {
    Active(Connection),
    Disabled,
    Unavailable(anyhow::Error),
}

impl Basemaps {
    pub fn load(directory: &Path) -> Result<Self> {
        let mut files = BTreeMap::new();
        for (id, path) in crate::enroute::storage::installed_files(directory)? {
            if !id.ends_with(".mbtiles") {
                continue;
            }
            let source = if path.with_extension("mbtiles.disabled").try_exists()? {
                BasemapSource::Disabled
            } else {
                match open_basemap(&path) {
                    Ok(connection) => BasemapSource::Active(connection),
                    Err(error) => BasemapSource::Unavailable(error),
                }
            };
            if let BasemapSource::Unavailable(error) = &source {
                tracing::warn!(%error, path = %path.display(), "Could not open offline basemap");
            }
            files.insert(id, source);
        }
        Ok(Self {
            files,
            directory: directory.to_owned(),
            ..Self::default()
        })
    }

    pub fn resource_response(&self, path: &str) -> Response<Vec<u8>> {
        let Some((generation, path)) = path.split_once('/') else {
            return response(StatusCode::BAD_REQUEST, Vec::new());
        };
        let Ok(generation) = generation.parse::<u64>() else {
            return response(StatusCode::BAD_REQUEST, Vec::new());
        };
        if generation != self.generation {
            return response(StatusCode::NO_CONTENT, Vec::new());
        }
        let Some([z, x, y]) = tile_coordinates(path) else {
            return response(StatusCode::BAD_REQUEST, Vec::new());
        };
        match self.tile(z, x, y) {
            Ok(Some(data)) => response(StatusCode::OK, data),
            Ok(None) => response(StatusCode::NO_CONTENT, Vec::new()),
            Err(error) => {
                tracing::warn!(?error, path, "Could not read offline basemap tile");
                response(StatusCode::INTERNAL_SERVER_ERROR, Vec::new())
            }
        }
    }

    fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        let path = self.directory.join(name);
        let source = self
            .files
            .get_mut(name)
            .context("Basemap file is not installed")?;
        let marker = path.with_extension("mbtiles.disabled");
        if enabled {
            match fs::remove_file(&marker) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            *source = match open_basemap(&path) {
                Ok(connection) => BasemapSource::Active(connection),
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "Could not open offline basemap");
                    BasemapSource::Unavailable(error)
                }
            };
        } else {
            match fs::File::create_new(&marker) {
                Ok(_) => {}
                Err(error)
                    if error.kind() == ErrorKind::AlreadyExists
                        && fs::symlink_metadata(&marker)?.is_file() => {}
                Err(error) => return Err(error.into()),
            }
            *source = BasemapSource::Disabled;
        }
        self.generation += 1;
        self.publish();
        Ok(())
    }

    pub fn install_download(&mut self, name: &str, download: BasemapDownload) -> Result<()> {
        let path = self.directory.join(name);
        ensure!(
            download.destination() == path,
            "Basemap download destination does not match"
        );
        if !self.files.contains_key(name) {
            let marker = path.with_extension("mbtiles.disabled");
            match fs::remove_file(marker) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let previous = self.files.remove(name);
        let installed = previous.is_some();
        let enabled = !matches!(previous, Some(BasemapSource::Disabled));
        // Close SQLite before replacement, including on Windows.
        drop(previous);
        let result = download.install();
        if result.is_ok() || installed {
            let source = if enabled {
                match open_basemap(&path) {
                    Ok(connection) => BasemapSource::Active(connection),
                    Err(error) => {
                        tracing::warn!(%error, name, "Could not open downloaded basemap");
                        BasemapSource::Unavailable(error)
                    }
                }
            } else {
                BasemapSource::Disabled
            };
            self.files.insert(name.to_owned(), source);
            self.generation += 1;
            self.publish();
        }
        result
    }

    fn remove(&mut self, name: &str) -> Result<()> {
        let path = self.directory.join(name);
        let source = self
            .files
            .get_mut(name)
            .context("Basemap file is not installed")?;
        let enabled = !matches!(source, BasemapSource::Disabled);
        // Close SQLite before deletion, including on Windows.
        *source = BasemapSource::Disabled;
        let marker = path.with_extension("mbtiles.disabled");
        let result =
            [&path, &marker]
                .into_iter()
                .try_for_each(|path| match fs::remove_file(path) {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                    Err(error) => Err(error),
                });
        if result.is_ok() {
            self.files.remove(name);
        } else if enabled {
            *source = open_basemap(&path)
                .map(BasemapSource::Active)
                .unwrap_or_else(BasemapSource::Unavailable);
        }
        self.generation += 1;
        self.publish();
        result.map_err(Into::into)
    }

    fn tile(&self, z: u32, x: u32, y: u32) -> Result<Option<Vec<u8>>> {
        let tms_y = (1_u32 << z) - 1 - y;
        for source in self.files.values() {
            let BasemapSource::Active(connection) = source else {
                continue;
            };
            let data: Option<Vec<u8>> = connection
                .prepare_cached(TILE_QUERY)?
                .query_row((z, x, tms_y), |row| row.get(0))
                .optional()
                .with_context(|| {
                    format!("Could not query {}", connection.path().unwrap_or_default())
                })?;
            if let Some(data) = data {
                let mut decoded = Vec::new();
                GzDecoder::new(data.as_slice()).read_to_end(&mut decoded)?;
                return Ok(Some(decoded));
            }
        }
        Ok(None)
    }
}

/// Serves a vector tile on a blocking worker.
pub async fn basemap_resource_response<R: tauri::Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Response<Vec<u8>> {
    let basemaps = app.state::<Arc<Mutex<Basemaps>>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let basemaps = basemaps.lock().expect("basemap access should not panic");
        basemaps.resource_response(&path)
    })
    .await
    .unwrap_or_else(|error| {
        tracing::warn!(%error, "Offline basemap resource worker failed");
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::new())
            .expect("the fixed error response should be valid")
    })
}

fn open_basemap(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let format_query = "SELECT value FROM metadata WHERE name = 'format'";
    let format: String = connection.query_row(format_query, [], |row| row.get(0))?;
    ensure!(format == "pbf", "Basemap format must be pbf");
    connection.prepare(TILE_QUERY)?;
    Ok(connection)
}

fn tile_coordinates(path: &str) -> Option<[u32; 3]> {
    let parts = path.strip_suffix(".pbf")?.split('/').collect::<Vec<_>>();
    let [z, x, y] = parts.as_slice() else {
        return None;
    };
    let [z, x, y] = [z.parse().ok()?, x.parse().ok()?, y.parse().ok()?];
    let size = 1_u32.checked_shl(z)?;
    (x < size && y < size).then_some([z, x, y])
}

fn response(status: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/vnd.mapbox-vector-tile")
        .header(header::CACHE_CONTROL, "no-store")
        .body(body)
        .expect("the fixed basemap response should be valid")
}

#[cfg(test)]
mod tests;

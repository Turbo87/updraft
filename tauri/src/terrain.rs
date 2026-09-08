use self::commands::TerrainStatus;
use crate::enroute::download::DownloadFile;
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
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

pub mod commands;

const TILE_QUERY: &str =
    "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3";

const TILE_METADATA_QUERY: &str = "
    SELECT tile_data,
           (SELECT min(zoom_level) FROM tiles), (SELECT max(zoom_level) FROM tiles)
    FROM tiles LIMIT 1";

#[derive(Default)]
pub struct Terrain {
    files: BTreeMap<String, TerrainSource>,
    directory: PathBuf,
    generation: u64,
    subscribers: BTreeMap<u32, Channel<TerrainStatus>>,
}

#[derive(Debug)]
enum TerrainSource {
    Active(TerrainFile),
    Disabled,
    Unavailable(anyhow::Error),
}

#[derive(Debug)]
struct TerrainFile {
    connection: Connection,
    coverage: Option<(usize, u32, u32)>,
    attributions: Vec<String>,
}

impl Terrain {
    pub fn load(directory: &Path) -> Result<Self> {
        let mut files = BTreeMap::new();
        let mut tile_size = None;
        for (id, path) in crate::enroute::storage::installed_files(directory)? {
            if !id.ends_with(".terrain") {
                continue;
            }
            let source = if path.with_extension("terrain.disabled").try_exists()? {
                TerrainSource::Disabled
            } else {
                match open_terrain(&path, tile_size) {
                    Ok(file) => {
                        if let Some((size, _, _)) = file.coverage {
                            tile_size = Some(size);
                        }
                        TerrainSource::Active(file)
                    }
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
            directory: directory.to_owned(),
            ..Self::default()
        })
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
            ("image/webp", self.tile(z, x, y))
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
        drop(previous);
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
        let mut tile_size = None;
        for (id, source) in &mut self.files {
            if !is_enabled(id, source) {
                *source = TerrainSource::Disabled;
                continue;
            }
            let path = self.directory.join(id);
            *source = match open_terrain(&path, tile_size) {
                Ok(file) => {
                    if let Some((size, _, _)) = file.coverage {
                        tile_size = Some(size);
                    }
                    TerrainSource::Active(file)
                }
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "Could not open offline terrain");
                    TerrainSource::Unavailable(error)
                }
            };
        }
        self.generation += 1;
        self.publish();
    }

    fn metadata(&self) -> Result<Vec<u8>> {
        let mut attributions = Vec::new();
        let mut coverage: Option<(usize, u32, u32)> = None;
        for source in self.files.values() {
            let TerrainSource::Active(file) = source else {
                continue;
            };
            if let Some((size, minzoom, maxzoom)) = file.coverage {
                match &mut coverage {
                    Some((_, min, max)) => {
                        *min = (*min).min(minzoom);
                        *max = (*max).max(maxzoom);
                    }
                    None => coverage = Some((size, minzoom, maxzoom)),
                }
            }
            for value in &file.attributions {
                if !attributions.contains(value) {
                    attributions.push(value.clone());
                }
            }
        }
        let generation = self.generation;
        let tiles = format!("updraft://localhost/terrain/{generation}/{{z}}/{{x}}/{{y}}.webp");
        let mut metadata = serde_json::json!({
            "tilejson": "3.0.0",
            "tiles": [tiles],
            "encoding": "terrarium",
            "attribution": attributions.join("<br>"),
        });
        if let Some((size, minzoom, maxzoom)) = coverage {
            metadata["tileSize"] = size.into();
            metadata["minzoom"] = minzoom.into();
            metadata["maxzoom"] = maxzoom.into();
        }
        Ok(serde_json::to_vec(&metadata)?)
    }

    fn tile(&self, z: u32, x: u32, y: u32) -> Result<Option<Vec<u8>>> {
        let tms_y = (1_u32 << z) - 1 - y;
        for source in self.files.values() {
            let TerrainSource::Active(file) = source else {
                continue;
            };
            let connection = &file.connection;
            let data = connection
                .prepare_cached(TILE_QUERY)?
                .query_row((z, x, tms_y), |row| row.get(0))
                .optional()
                .with_context(|| {
                    format!("Could not query {}", connection.path().unwrap_or_default())
                })?;
            if data.is_some() {
                return Ok(data);
            }
        }
        Ok(None)
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

fn open_terrain(path: &Path, tile_size: Option<usize>) -> Result<TerrainFile> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    for (name, expected) in [("format", "webp"), ("encoding", "terrarium")] {
        let query = "SELECT value FROM metadata WHERE name = ?1";
        let value: String = connection.query_row(query, [name], |row| row.get(0))?;
        ensure!(value == expected, "Terrain {name} must be {expected}");
    }
    connection.prepare(TILE_QUERY)?;
    let tile_metadata: Option<(Vec<u8>, u32, u32)> = connection
        .prepare_cached(TILE_METADATA_QUERY)?
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .optional()?;
    let coverage = if let Some((data, minzoom, maxzoom)) = tile_metadata {
        ensure!(
            imagesize::image_type(&data)? == imagesize::ImageType::Webp,
            "Terrain tiles must use WebP"
        );
        let imagesize::ImageSize { width, height } = imagesize::blob_size(&data)?;
        ensure!(width == height, "Terrain tiles must be square");
        ensure!(maxzoom < 32, "Terrain zoom levels must be below 32");
        ensure!(
            tile_size.is_none_or(|size| size == width),
            "Terrain files must use the same tile size"
        );
        Some((width, minzoom, maxzoom))
    } else {
        None
    };
    let mut attributions = Vec::new();
    let query = "SELECT value FROM metadata WHERE name = 'attribution' ORDER BY rowid";
    let mut statement = connection.prepare_cached(query)?;
    for value in statement.query_map([], |row| row.get::<_, String>(0))? {
        let value = value?.trim().to_owned();
        if !value.is_empty() && value != "None yet" && !attributions.contains(&value) {
            attributions.push(value);
        }
    }
    drop(statement);
    Ok(TerrainFile {
        connection,
        coverage,
        attributions,
    })
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

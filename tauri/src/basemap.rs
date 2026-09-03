use anyhow::{Context, Result, ensure};
use flate2::read::GzDecoder;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::io::{ErrorKind, Read};
use std::sync::{Arc, Mutex};
use std::{fs, path::Path};
use tauri::http::{Response, StatusCode, header};
use tauri::{AppHandle, Manager};

const TILE_QUERY: &str =
    "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3";

#[derive(Default)]
pub struct Basemaps {
    files: Vec<Connection>,
}

impl Basemaps {
    pub fn load(directory: &Path) -> Result<Self> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error.into()),
        };
        let mut paths = entries
            .map(|entry| entry.map(|entry| entry.path()))
            .filter(|path| match path {
                Ok(path) => path
                    .extension()
                    .is_some_and(|extension| extension == "mbtiles"),
                Err(_) => true,
            })
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.sort();
        let mut files = Vec::new();
        for path in paths {
            match open_basemap(&path) {
                Ok(connection) => files.push(connection),
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "Could not open offline basemap");
                }
            }
        }
        Ok(Self { files })
    }

    pub fn resource_response(&self, path: &str) -> Response<Vec<u8>> {
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

    fn tile(&self, z: u32, x: u32, y: u32) -> Result<Option<Vec<u8>>> {
        let tms_y = (1_u32 << z) - 1 - y;
        for connection in &self.files {
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

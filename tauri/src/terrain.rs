use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::sync::{Arc, Mutex};
use std::{fs, io::ErrorKind, path::Path};
use tauri::http::{Response, StatusCode, header};
use tauri::{AppHandle, Manager};

const TILE_QUERY: &str =
    "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3";

const TILE_METADATA_QUERY: &str = "
    SELECT tile_data,
           (SELECT min(zoom_level) FROM tiles), (SELECT max(zoom_level) FROM tiles)
    FROM tiles LIMIT 1";

#[derive(Default)]
pub struct Terrain {
    files: Vec<Connection>,
}

impl Terrain {
    pub fn load(directory: &Path) -> Result<Self> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error.into()),
        };
        let mut paths = Vec::new();
        for entry in entries {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "terrain")
            {
                paths.push(path);
            }
        }
        paths.sort();
        let mut files = Vec::new();
        for path in paths {
            match open_terrain(&path) {
                Ok(connection) => files.push(connection),
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "Could not open offline terrain");
                }
            }
        }
        Ok(Self { files })
    }

    pub fn resource_response(&self, path: &str) -> Response<Vec<u8>> {
        let (content_type, result) = if path == "metadata.json" {
            ("application/json", self.metadata().map(Some))
        } else if let Some([z, x, y]) = terrain_coordinates(path) {
            ("image/webp", self.tile(z, x, y))
        } else {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Vec::new())
                .expect("the fixed error response should be valid");
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

    fn metadata(&self) -> Result<Vec<u8>> {
        let mut attributions = Vec::new();
        let mut coverage: Option<(usize, u32, u32)> = None;
        for connection in &self.files {
            let tile_metadata: Option<(Vec<u8>, u32, u32)> = connection
                .prepare_cached(TILE_METADATA_QUERY)?
                .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .optional()?;
            if let Some((data, minzoom, maxzoom)) = tile_metadata {
                ensure!(
                    imagesize::image_type(&data)? == imagesize::ImageType::Webp,
                    "Terrain tiles must use WebP"
                );
                let imagesize::ImageSize { width, height } = imagesize::blob_size(&data)?;
                ensure!(width == height, "Terrain tiles must be square");
                ensure!(maxzoom < 32, "Terrain zoom levels must be below 32");
                match &mut coverage {
                    Some((size, min, max)) => {
                        ensure!(*size == width, "Terrain files must use the same tile size");
                        *min = (*min).min(minzoom);
                        *max = (*max).max(maxzoom);
                    }
                    None => coverage = Some((width, minzoom, maxzoom)),
                }
            }
            let query = "SELECT value FROM metadata WHERE name = 'attribution' ORDER BY rowid";
            let mut statement = connection.prepare_cached(query)?;
            for value in statement.query_map([], |row| row.get::<_, String>(0))? {
                let value = value?.trim().to_owned();
                if !value.is_empty() && value != "None yet" && !attributions.contains(&value) {
                    attributions.push(value);
                }
            }
        }
        let mut metadata = serde_json::json!({
            "tilejson": "3.0.0",
            "tiles": ["updraft://localhost/terrain/{z}/{x}/{y}.webp"],
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
        for connection in &self.files {
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

fn open_terrain(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    for (name, expected) in [("format", "webp"), ("encoding", "terrarium")] {
        let query = "SELECT value FROM metadata WHERE name = ?1";
        let value: String = connection.query_row(query, [name], |row| row.get(0))?;
        ensure!(value == expected, "Terrain {name} must be {expected}");
    }
    connection.prepare(TILE_QUERY)?;
    Ok(connection)
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

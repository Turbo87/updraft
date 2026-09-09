//! Reads offline Terrarium WebP databases and samples terrain MSL elevation.
//!
//! Callers select files and run blocking reads. This crate does not manage
//! downloads, activation settings, runtime tasks, or instrument state.

use self::dem::{TerrainTile, bilinear};
use anyhow::{Context, Result, ensure};
use imagesize::{ImageSize, ImageType};
use moka::sync::Cache;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::{collections::BTreeMap, f64::consts::PI, path::Path, sync::Arc};
use updraft_geo::LatLon;

mod dem;

const TILE_QUERY: &str =
    "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3";

const TILE_METADATA_QUERY: &str = "
    SELECT tile_data,
           (SELECT min(zoom_level) FROM tiles), (SELECT max(zoom_level) FROM tiles)
    FROM tiles LIMIT 1";

/// Selected terrain files and their decoded tile cache.
///
/// Files are read in lexical source-name order. Source changes invalidate the
/// cache. File contents must remain unchanged until removal or replacement.
#[derive(Debug)]
pub struct TerrainReader {
    files: BTreeMap<String, TerrainFile>,
    decoded: Cache<(u32, u32, u32), Option<Arc<TerrainTile>>>,
}

/// Validated tile dimensions, zoom limits, and attribution from selected files.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TerrainMetadata {
    pub coverage: Option<TerrainCoverage>,
    pub attributions: Vec<String>,
}

/// The common square tile size and combined inclusive zoom range.
#[derive(Debug, PartialEq, Eq)]
pub struct TerrainCoverage {
    pub tile_size: usize,
    pub min_zoom: u32,
    pub max_zoom: u32,
}

#[derive(Debug)]
struct TerrainFile {
    connection: Connection,
    coverage: Option<(usize, u32, u32)>,
    attributions: Vec<String>,
}

impl Default for TerrainReader {
    fn default() -> Self {
        Self {
            files: BTreeMap::new(),
            decoded: Cache::builder()
                .max_capacity(16 * 1024 * 1024)
                .weigher(|_, tile: &Option<Arc<TerrainTile>>| {
                    tile.as_ref().map_or(1, |tile| tile.pixels.len() as u32)
                })
                .build(),
        }
    }
}

impl TerrainReader {
    /// Opens a file read-only and inserts it under the supplied source name.
    ///
    /// Validation failures leave the selected files unchanged. Files with tiles
    /// must have the same tile size. Remove a source before changing its bytes
    /// on disk so its SQLite handle is closed, including on Windows.
    pub fn insert(&mut self, name: String, path: &Path) -> Result<()> {
        let tile_size = self
            .files
            .iter()
            .filter(|(id, _)| **id != name)
            .find_map(|(_, file)| file.coverage.map(|(size, _, _)| size));
        let file = open_terrain(path, tile_size)?;
        self.files.insert(name, file);
        self.decoded.invalidate_all();
        Ok(())
    }

    /// Closes the selected source and invalidates cached tiles, including misses.
    pub fn remove(&mut self, name: &str) {
        self.files.remove(name);
        self.decoded.invalidate_all();
    }

    /// Closes all source files and discards cached tiles, including misses.
    pub fn clear(&mut self) {
        self.files.clear();
        self.decoded.invalidate_all();
    }

    /// Combines validated metadata without reading the files again.
    pub fn metadata(&self) -> TerrainMetadata {
        let mut attributions = Vec::new();
        let mut coverage: Option<(usize, u32, u32)> = None;
        for file in self.files.values() {
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
        TerrainMetadata {
            coverage: coverage.map(|(tile_size, min_zoom, max_zoom)| TerrainCoverage {
                tile_size,
                min_zoom,
                max_zoom,
            }),
            attributions,
        }
    }

    /// Samples terrain MSL elevation in meters at the highest available zoom.
    ///
    /// Returns `None` outside coverage or Web Mercator latitude bounds. Read
    /// and decode failures return an error. Interpolation uses adjacent tiles
    /// when available and repeats edge samples where coverage ends.
    pub fn elevation(&self, position: LatLon) -> Result<Option<f64>> {
        let lat = position.latitude().as_degrees();
        let lon = position.longitude().as_degrees();
        if !lat.is_finite() || lat.abs() > 85.0511287798066 || !lon.is_finite() {
            return Ok(None);
        }
        let mx = (lon + 180.0).rem_euclid(360.0) / 360.0;
        let my = (1.0 - position.latitude().as_radians().tan().asinh() / PI) / 2.0;
        for z in (0..32).rev().filter(|z| {
            self.files.values().any(|source| {
                source
                    .coverage
                    .is_some_and(|(_, min, max)| (min..=max).contains(z))
            })
        }) {
            let count = 1_u32 << z;
            let tx = mx * f64::from(count);
            let ty = (my * f64::from(count))
                .clamp(0.0, f64::from(count) - f64::EPSILON * f64::from(count));
            let (x, y) = (tx.floor() as u32, ty.floor() as u32);
            let Some(tile) = self.decoded_tile(z, x, y)? else {
                continue;
            };
            // Elevation samples lie at pixel centers.
            let px = (tx - f64::from(x)) * f64::from(tile.size) - 0.5;
            let py = (ty - f64::from(y)) * f64::from(tile.size) - 0.5;
            let decoded_neighbour = |nx, ny| {
                if (nx, ny) == (x, y) {
                    Ok(Some(tile.clone()))
                } else {
                    self.decoded_tile(z, nx, ny)
                }
            };
            let mut samples = [[0.0; 2]; 2];
            for (dy, row) in samples.iter_mut().enumerate() {
                for (dx, sample) in row.iter_mut().enumerate() {
                    let ix = px.floor() as i64 + dx as i64;
                    let iy = py.floor() as i64 + dy as i64;
                    let size = i64::from(tile.size);
                    let nx =
                        (i64::from(x) + ix.div_euclid(size)).rem_euclid(i64::from(count)) as u32;
                    let ny = i64::from(y) + iy.div_euclid(size);
                    let mut neighbour = if (0..i64::from(count)).contains(&ny) {
                        decoded_neighbour(nx, ny as u32)?
                    } else {
                        None
                    };
                    let mut sx = ix.rem_euclid(size) as u32;
                    let mut sy = iy.rem_euclid(size) as u32;
                    if neighbour.is_none() {
                        neighbour = decoded_neighbour(nx, y)?;
                        sy = iy.clamp(0, size - 1) as u32;
                    }
                    if neighbour.is_none() && (0..i64::from(count)).contains(&ny) {
                        neighbour = decoded_neighbour(x, ny as u32)?;
                        sx = ix.clamp(0, size - 1) as u32;
                        sy = iy.rem_euclid(size) as u32;
                    }
                    *sample = match neighbour {
                        Some(neighbour) => {
                            ensure!(
                                neighbour.size == tile.size,
                                "Terrain tiles must use the same size"
                            );
                            neighbour.elevation(sx, sy)
                        }
                        None => tile
                            .elevation(ix.clamp(0, size - 1) as u32, iy.clamp(0, size - 1) as u32),
                    };
                }
            }
            let elevation = bilinear(samples, px - px.floor(), py - py.floor());
            return Ok(Some(elevation));
        }
        Ok(None)
    }

    fn decoded_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<Arc<TerrainTile>>> {
        self.decoded
            .try_get_with((z, x, y), || {
                self.tile(z, x, y)?
                    .map(|bytes| TerrainTile::decode(&bytes).map(Arc::new))
                    .transpose()
            })
            .map_err(|error: Arc<anyhow::Error>| {
                anyhow::anyhow!("Could not decode terrain tile {z}/{x}/{y}: {error:#}")
            })
    }

    /// Reads unchanged WebP bytes using XYZ tile coordinates.
    ///
    /// Returns `None` outside coverage. Invalid coordinates and database read
    /// failures return an error.
    pub fn tile(&self, z: u32, x: u32, y: u32) -> Result<Option<Vec<u8>>> {
        let size = 1_u32
            .checked_shl(z)
            .context("Terrain zoom level must be below 32")?;
        ensure!(
            x < size && y < size,
            "Terrain tile coordinates exceed the zoom bounds"
        );
        let tms_y = size - 1 - y;
        for file in self.files.values() {
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
        let image_type = imagesize::image_type(&data)?;
        ensure!(image_type == ImageType::Webp, "Terrain tiles must use WebP");
        let ImageSize { width, height } = imagesize::blob_size(&data)?;
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

#[cfg(test)]
mod tests;

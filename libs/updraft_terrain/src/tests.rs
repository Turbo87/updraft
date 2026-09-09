use super::*;
use claims::{assert_err, assert_none, assert_ok, assert_some, assert_some_eq};
use rusqlite::Connection;
use std::path::Path;
use updraft_geo::LatLon;

fn write_terrain(path: &Path, tiles: &[(u32, u32, u32, &[u8])]) {
    let connection = Connection::open(path).unwrap();
    connection.execute_batch(
        "CREATE TABLE metadata (name TEXT, value TEXT);
         INSERT INTO metadata VALUES ('format', 'webp'), ('encoding', 'terrarium');
         CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);
         CREATE UNIQUE INDEX tile_index ON tiles (zoom_level, tile_column, tile_row);",
    ).unwrap();
    for &(z, x, tms_y, data) in tiles {
        connection
            .execute(
                "INSERT INTO tiles VALUES (?1, ?2, ?3, ?4)",
                (z, x, tms_y, data),
            )
            .unwrap();
    }
}

fn elevation_webp(meters: u16) -> Vec<u8> {
    let value = meters + 32768;
    let pixel = [(value >> 8) as u8, value as u8, 0];
    let mut bytes = Vec::new();
    image_webp::WebPEncoder::new(&mut bytes)
        .encode(&pixel.repeat(4), 2, 2, image_webp::ColorType::Rgb8)
        .unwrap();
    bytes
}

#[test]
fn source_changes_invalidate_decoded_and_missing_tiles() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.terrain");
    let second = directory.path().join("second.terrain");
    write_terrain(&first, &[(1, 0, 1, &elevation_webp(100))]);
    let high = elevation_webp(200);
    write_terrain(&second, &[(1, 0, 1, &high), (1, 1, 1, &high)]);
    let mut terrain = TerrainReader::default();
    let west = LatLon::from_degrees(40.0, -60.0);
    let east = LatLon::from_degrees(40.0, 60.0);
    assert_ok!(terrain.insert("a".into(), &first));
    assert_some_eq!(assert_ok!(terrain.elevation(west)), 100.0);
    assert_none!(assert_ok!(terrain.elevation(east)));
    assert_ok!(terrain.insert("b".into(), &second));
    assert_some_eq!(assert_ok!(terrain.elevation(east)), 200.0);
    assert_some_eq!(assert_ok!(terrain.elevation(west)), 100.0);
    terrain.remove("a");
    assert_some_eq!(assert_ok!(terrain.elevation(west)), 200.0);
    assert_ok!(terrain.insert("a".into(), &first));
    assert_some_eq!(assert_ok!(terrain.elevation(west)), 100.0);
    assert_ok!(terrain.insert("a".into(), &second));
    assert_some_eq!(assert_ok!(terrain.elevation(west)), 200.0);
    terrain.remove("a");
    assert_some_eq!(assert_ok!(terrain.elevation(east)), 200.0);
    terrain.remove("b");
    assert_none!(assert_ok!(terrain.elevation(east)));
    assert_ok!(terrain.insert("a".into(), &first));
    assert_some_eq!(assert_ok!(terrain.elevation(west)), 100.0);
    terrain.clear();
    assert_none!(assert_ok!(terrain.elevation(west)));
    assert_none!(assert_ok!(terrain.tile(1, 0, 0)));
}

#[test]
fn failed_insertion_retains_previous_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("valid.terrain");
    let tile = elevation_webp(100);
    write_terrain(&path, &[(0, 0, 0, &tile)]);
    let mut terrain = TerrainReader::default();
    assert_ok!(terrain.insert("a".into(), &path));
    assert_err!(terrain.insert("a".into(), &directory.path().join("missing.terrain")));
    assert_some_eq!(assert_ok!(terrain.tile(0, 0, 0)), tile);
}

#[test]
fn tile_reads_reject_invalid_coordinates() {
    let terrain = TerrainReader::default();
    for (z, x, y) in [(32, 0, 0), (7, 128, 0), (7, 0, 128)] {
        assert_err!(terrain.tile(z, x, y));
    }
}

#[test]
fn elevation_selects_highest_available_zoom_and_caches_decoding() {
    let directory = tempfile::tempdir().unwrap();
    let low = elevation_webp(100);
    let high = elevation_webp(500);
    write_terrain(&directory.path().join("a.terrain"), &[(0, 0, 0, &low)]);
    write_terrain(&directory.path().join("b.terrain"), &[(1, 1, 0, &high)]);
    let mut terrain = TerrainReader::default();
    assert_ok!(terrain.insert("a".into(), &directory.path().join("a.terrain")));
    assert_ok!(terrain.insert("b".into(), &directory.path().join("b.terrain")));
    let position = updraft_geo::LatLon::from_degrees(-40.0, 60.0);
    assert_some_eq!(assert_ok!(terrain.elevation(position)), 500.0);
    let first = assert_some!(assert_ok!(terrain.decoded_tile(1, 1, 1)));
    let second = assert_some!(assert_ok!(terrain.decoded_tile(1, 1, 1)));
    assert!(Arc::ptr_eq(&first, &second));
    assert_some_eq!(
        assert_ok!(terrain.elevation(updraft_geo::LatLon::from_degrees(40.0, -60.0))),
        100.0
    );
}

#[test]
fn elevation_distinguishes_missing_coverage_and_read_errors() {
    let terrain = TerrainReader::default();
    assert_none!(assert_ok!(
        terrain.elevation(updraft_geo::LatLon::from_degrees(0.0, 0.0))
    ));
    assert_none!(assert_ok!(
        terrain.elevation(updraft_geo::LatLon::from_degrees(90.0, 0.0))
    ));
    assert_none!(assert_ok!(
        terrain.elevation(updraft_geo::LatLon::from_degrees(f64::NAN, 0.0))
    ));
    let directory = tempfile::tempdir().unwrap();
    write_terrain(
        &directory.path().join("broken.terrain"),
        &[(0, 0, 0, &elevation_webp(100))],
    );
    let mut terrain = TerrainReader::default();
    assert_ok!(terrain.insert("broken".into(), &directory.path().join("broken.terrain")));
    Connection::open(directory.path().join("broken.terrain"))
        .unwrap()
        .execute("UPDATE tiles SET tile_data = ?1", [b"broken".as_slice()])
        .unwrap();
    assert_err!(terrain.elevation(updraft_geo::LatLon::from_degrees(0.0, 0.0)));
}

#[test]
fn elevation_interpolates_across_tile_edges() {
    let directory = tempfile::tempdir().unwrap();
    let west = elevation_webp(100);
    let east = elevation_webp(200);
    write_terrain(
        &directory.path().join("a.terrain"),
        &[(1, 0, 1, &west), (1, 1, 1, &east)],
    );
    let mut terrain = TerrainReader::default();
    assert_ok!(terrain.insert("a".into(), &directory.path().join("a.terrain")));
    assert_some_eq!(
        assert_ok!(terrain.elevation(updraft_geo::LatLon::from_degrees(40.0, 0.0))),
        150.0
    );
}

#[test]
fn reads_unchanged_bytes_in_source_name_order_with_xyz_coordinates() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.terrain");
    let second = directory.path().join("second.terrain");
    let low = elevation_webp(100);
    let high = elevation_webp(200);
    write_terrain(&first, &[(7, 66, 87, &low), (10, 0, 0, &low)]);
    write_terrain(&second, &[(7, 66, 87, &high), (10, 1023, 1023, &high)]);
    let mut terrain = TerrainReader::default();
    assert_ok!(terrain.insert("b".into(), &first));
    assert_ok!(terrain.insert("a".into(), &second));
    assert_some_eq!(assert_ok!(terrain.tile(7, 66, 40)), high.clone());
    assert_some_eq!(assert_ok!(terrain.tile(10, 0, 1023)), low);
    assert_some_eq!(assert_ok!(terrain.tile(10, 1023, 0)), high);
    assert_none!(assert_ok!(terrain.tile(7, 0, 0)));
}

#[test]
fn rejects_unsupported_or_inconsistent_tile_metadata() {
    for (tile, zoom) in [
        (b"not a WebP header".to_vec(), 7),
        (webp_header(256, 512), 7),
        (webp_header(512, 512), 7),
        (webp_header(256, 256), 32),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.terrain");
        let second = directory.path().join("second.terrain");
        write_terrain(&first, &[(7, 0, 0, &webp_header(256, 256))]);
        write_terrain(&second, &[(zoom, 1, 0, &tile)]);
        let mut terrain = TerrainReader::default();
        assert_ok!(terrain.insert("a".into(), &first));
        let metadata = terrain.metadata();
        assert_err!(terrain.insert("b".into(), &second));
        assert_eq!(terrain.metadata(), metadata);
        if zoom < 32 {
            assert_none!(assert_ok!(terrain.tile(zoom, 1, (1_u32 << zoom) - 1)));
        }
    }
}

fn webp_header(width: u32, height: u32) -> Vec<u8> {
    let mut header = b"RIFF\x11\0\0\0WEBPVP8L\x05\0\0\0\x2f".to_vec();
    header.extend_from_slice(&((width - 1) | ((height - 1) << 14)).to_le_bytes());
    header
}

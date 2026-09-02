use super::*;
use claims::assert_ok;
use flate2::{Compression, write::GzEncoder};
use rusqlite::Connection;
use std::{io::Write, path::Path};
use tauri::http::{StatusCode, header};

fn write_basemap(path: &Path, tiles: &[(u32, u32, u32, &[u8])]) {
    let connection = Connection::open(path).unwrap();
    connection.execute_batch(
        "CREATE TABLE metadata (name TEXT, value TEXT);
         INSERT INTO metadata VALUES ('format', 'pbf'), ('minzoom', '6'), ('maxzoom', '10');
         CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);
         CREATE UNIQUE INDEX tile_index ON tiles (zoom_level, tile_column, tile_row);",
    ).unwrap();
    for &(zoom, x, tms_y, data) in tiles {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let data = encoder.finish().unwrap();
        let insert_tile = "INSERT INTO tiles VALUES (?1, ?2, ?3, ?4)";
        connection
            .execute(insert_tile, (zoom, x, tms_y, data))
            .unwrap();
    }
}

#[test]
fn serves_the_first_tile_in_filename_order_with_xyz_coordinates() {
    let directory = tempfile::tempdir().unwrap();
    let germany = directory.path().join("Germany.mbtiles");
    write_basemap(&germany, &[(6, 33, 43, b"germany")]);
    let france = directory.path().join("France.mbtiles");
    write_basemap(&france, &[(6, 33, 43, b"france")]);

    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    let response = basemaps.resource_response("6/33/20.pbf");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = &response.headers()[header::CONTENT_TYPE];
    assert_eq!(content_type, "application/vnd.mapbox-vector-tile");
    assert_eq!(response.body(), b"france");
}

#[test]
fn serves_tiles_without_zoom_or_attribution_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("minimal.mbtiles");
    write_basemap(&path, &[(6, 33, 43, b"tile")]);
    Connection::open(path)
        .unwrap()
        .execute("DELETE FROM metadata WHERE name != 'format'", [])
        .unwrap();

    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(basemaps.resource_response("6/33/20.pbf").body(), b"tile");
}

#[test]
#[tracing_test::traced_test]
fn skips_invalid_files_and_continues_with_valid_basemaps() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("broken.mbtiles"), b"not sqlite").unwrap();
    let raster_format = "UPDATE metadata SET value = 'png' WHERE name = 'format'";
    for (name, update) in [("raster", raster_format), ("schema", "DROP TABLE tiles")] {
        let path = directory.path().join(format!("{name}.mbtiles"));
        write_basemap(&path, &[]);
        Connection::open(path)
            .unwrap()
            .execute_batch(update)
            .unwrap();
    }
    let valid = directory.path().join("valid.mbtiles");
    write_basemap(&valid, &[(6, 33, 43, b"valid")]);
    let ignored = directory.path().join("ignored.sqlite");
    write_basemap(&ignored, &[(6, 33, 43, b"ignored")]);

    let basemaps = assert_ok!(Basemaps::load(directory.path()));

    assert_eq!(basemaps.resource_response("6/33/20.pbf").body(), b"valid");
    for name in ["broken.mbtiles", "raster.mbtiles", "schema.mbtiles"] {
        assert!(logs_contain(name));
    }
    assert!(logs_contain("Could not open offline basemap"));
}

#[test]
fn missing_directory_has_no_tiles() {
    let directory = tempfile::tempdir().unwrap();
    let basemaps = assert_ok!(Basemaps::load(&directory.path().join("enroute")));

    let response = basemaps.resource_response("6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.body().is_empty());
}

#[test]
fn looks_in_later_files_and_serves_both_antimeridian_columns() {
    let directory = tempfile::tempdir().unwrap();
    let east = directory.path().join("east.mbtiles");
    write_basemap(&east, &[(6, 63, 63, b"east")]);
    let west = directory.path().join("west.mbtiles");
    write_basemap(&west, &[(6, 0, 0, b"west")]);
    for name in ["east", "west"] {
        let path = directory.path().join(format!("{name}.mbtiles"));
        let insert_bounds = "INSERT INTO metadata VALUES ('bounds', '170,-20,-170,20')";
        Connection::open(path)
            .unwrap()
            .execute(insert_bounds, [])
            .unwrap();
    }
    let basemaps = assert_ok!(Basemaps::load(directory.path()));

    assert_eq!(basemaps.resource_response("6/63/0.pbf").body(), b"east");
    assert_eq!(basemaps.resource_response("6/0/63.pbf").body(), b"west");
    let response = basemaps.resource_response("6/1/1.pbf");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[test]
fn rejects_invalid_tile_coordinates_without_querying_files() {
    let basemaps = Basemaps::default();
    for path in [
        "32/0/0.pbf",
        "6/64/0.pbf",
        "6/0/64.pbf",
        "6/-1/0.pbf",
        "6/0.pbf",
        "6/0/0/0.pbf",
        "a/0/0.pbf",
        "6/0/0.png",
    ] {
        let response = basemaps.resource_response(path);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
    }
}

#[test]
#[tracing_test::traced_test]
fn reports_database_and_gzip_failures_as_errors_instead_of_missing_tiles() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("corrupt.mbtiles");
    write_basemap(&path, &[(6, 33, 43, b"tile")]);
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute("UPDATE tiles SET tile_data = ?1", [b"not gzip".as_slice()])
        .unwrap();

    let response = basemaps.resource_response("6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    connection.execute("DROP TABLE tiles", []).unwrap();
    let response = basemaps.resource_response("6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(logs_contain("Could not read offline basemap tile"));
}

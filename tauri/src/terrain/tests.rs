use super::*;
use claims::{assert_err, assert_ok};
use rusqlite::Connection;
use tauri::http::header;

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

#[test]
fn serves_unchanged_bytes_in_filename_order_with_xyz_coordinates() {
    let directory = tempfile::tempdir().unwrap();
    write_terrain(
        &directory.path().join("Germany.terrain"),
        &[(7, 66, 87, b"germany"), (10, 0, 0, b"west")],
    );
    write_terrain(
        &directory.path().join("France.terrain"),
        &[(7, 66, 87, b"france"), (10, 1023, 1023, b"east")],
    );
    write_terrain(
        &directory.path().join("Basemap.mbtiles"),
        &[(7, 66, 87, b"ignored")],
    );
    let terrain = assert_ok!(Terrain::load(directory.path()));

    let response = terrain.resource_response("7/66/40.webp");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/webp");
    assert_eq!(response.body(), b"france");
    assert_eq!(terrain.resource_response("10/0/1023.webp").body(), b"west");
    assert_eq!(terrain.resource_response("10/1023/0.webp").body(), b"east");
    assert_eq!(
        terrain.resource_response("7/0/0.webp").status(),
        StatusCode::NOT_FOUND
    );
}

#[test]
#[tracing_test::traced_test]
fn skips_invalid_databases_formats_and_encodings() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("broken.terrain"), b"not sqlite").unwrap();
    for (name, sql) in [
        (
            "format",
            "UPDATE metadata SET value = 'png' WHERE name = 'format'",
        ),
        (
            "encoding",
            "UPDATE metadata SET value = 'mapbox' WHERE name = 'encoding'",
        ),
        ("missing", "DELETE FROM metadata WHERE name = 'encoding'"),
        ("schema", "DROP TABLE tiles"),
    ] {
        let path = directory.path().join(format!("{name}.terrain"));
        write_terrain(&path, &[]);
        Connection::open(path).unwrap().execute_batch(sql).unwrap();
    }
    write_terrain(
        &directory.path().join("valid.terrain"),
        &[(7, 66, 87, b"valid")],
    );
    let terrain = assert_ok!(Terrain::load(directory.path()));
    assert_eq!(terrain.resource_response("7/66/40.webp").body(), b"valid");
    for name in ["broken", "format", "encoding", "missing", "schema"] {
        assert!(logs_contain(&format!("{name}.terrain")));
    }
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
fn missing_directory_is_empty_but_scan_failure_is_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("enroute");
    let terrain = assert_ok!(Terrain::load(&path));
    let response = terrain.resource_response("7/66/40.webp");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.body().is_empty());
    std::fs::write(&path, b"not a directory").unwrap();
    assert_err!(Terrain::load(&path).map(|_| ()));
}

#[test]
fn rejects_invalid_requests() {
    let terrain = Terrain::default();
    for path in [
        "32/0/0.webp",
        "7/128/0.webp",
        "7/0/128.webp",
        "7/-1/0.webp",
        "7/0.webp",
        "7/0/0/0.webp",
        "a/0/0.webp",
        "7/0/0.pbf",
    ] {
        assert_eq!(
            terrain.resource_response(path).status(),
            StatusCode::BAD_REQUEST,
            "{path}"
        );
    }
}

#[test]
#[tracing_test::traced_test]
fn reports_read_failures_instead_of_missing_coverage() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Germany.terrain");
    write_terrain(&path, &[]);
    let terrain = assert_ok!(Terrain::load(directory.path()));
    let connection = Connection::open(path).unwrap();
    connection.execute("DROP TABLE tiles", []).unwrap();
    assert_eq!(
        terrain.resource_response("7/66/40.webp").status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(logs_contain("Could not read offline terrain resource"));
    connection.execute("DROP TABLE metadata", []).unwrap();
    assert_eq!(
        terrain.resource_response("metadata.json").status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn serves_tilejson_with_combined_attributions_without_duplicates_or_placeholders() {
    let directory = tempfile::tempdir().unwrap();
    let mut extended_header = b"RIFF\x16\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
    extended_header.extend_from_slice(&511_u32.to_le_bytes()[..3]);
    extended_header.extend_from_slice(&511_u32.to_le_bytes()[..3]);
    for (country, minzoom, maxzoom, tile) in [
        ("Germany", 5, 12, webp_header(512, 512)),
        ("France", 4, 11, extended_header),
    ] {
        let path = directory.path().join(format!("{country}.terrain"));
        write_terrain(&path, &[(minzoom, 0, 0, &tile), (maxzoom, 0, 0, &tile)]);
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "INSERT INTO metadata VALUES ('attribution', 'Shared credit'),
             ('attribution', 'None yet'), ('attribution', '  ');",
            )
            .unwrap();
        connection
            .execute("INSERT INTO metadata VALUES ('attribution', ?1)", [country])
            .unwrap();
    }
    let terrain = assert_ok!(Terrain::load(directory.path()));
    let response = terrain.resource_response("metadata.json");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    let metadata: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    insta::assert_json_snapshot!(metadata, @r#"
    {
      "attribution": "Shared credit<br>France<br>Germany",
      "encoding": "terrarium",
      "maxzoom": 12,
      "minzoom": 4,
      "tileSize": 512,
      "tilejson": "3.0.0",
      "tiles": [
        "updraft://localhost/terrain/{z}/{x}/{y}.webp"
      ]
    }
    "#);
}

fn webp_header(width: u32, height: u32) -> Vec<u8> {
    let mut header = b"RIFF\x11\0\0\0WEBPVP8L\x05\0\0\0\x2f".to_vec();
    header.extend_from_slice(&((width - 1) | ((height - 1) << 14)).to_le_bytes());
    header
}

#[test]
fn empty_files_do_not_add_tile_dimensions_or_zoom_limits() {
    let directory = tempfile::tempdir().unwrap();
    write_terrain(&directory.path().join("empty.terrain"), &[]);
    let terrain = assert_ok!(Terrain::load(directory.path()));
    let response = terrain.resource_response("metadata.json");
    assert_eq!(response.status(), StatusCode::OK);
    let metadata: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    insta::assert_json_snapshot!(metadata, @r#"
    {
      "attribution": "",
      "encoding": "terrarium",
      "tilejson": "3.0.0",
      "tiles": [
        "updraft://localhost/terrain/{z}/{x}/{y}.webp"
      ]
    }
    "#);
    assert_eq!(
        response.body(),
        Terrain::default().resource_response("metadata.json").body()
    );
}

#[test]
#[tracing_test::traced_test]
fn rejects_unsupported_or_inconsistent_tile_metadata() {
    for (tile, zoom) in [
        (b"not a WebP header".to_vec(), 7),
        (webp_header(256, 512), 7),
        (webp_header(512, 512), 7),
        (webp_header(256, 256), 32),
    ] {
        let directory = tempfile::tempdir().unwrap();
        write_terrain(
            &directory.path().join("France.terrain"),
            &[(7, 0, 0, &webp_header(256, 256))],
        );
        write_terrain(
            &directory.path().join("Germany.terrain"),
            &[(zoom, 0, 0, &tile)],
        );
        let terrain = assert_ok!(Terrain::load(directory.path()));
        assert_eq!(
            terrain.resource_response("metadata.json").status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    assert!(logs_contain("Could not read offline terrain resource"));
}

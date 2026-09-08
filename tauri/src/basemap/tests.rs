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

#[test]
#[tracing_test::traced_test]
fn inventory_retains_invalid_files_and_does_not_open_disabled_files() {
    let directory = tempfile::tempdir().unwrap();
    let broken = directory.path().join("broken.mbtiles");
    let disabled = directory.path().join("disabled.mbtiles");
    let valid = directory.path().join("valid.mbtiles");
    assert_ok!(fs::write(&broken, b"not sqlite"));
    assert_ok!(fs::write(&disabled, b"also not sqlite"));
    assert_ok!(fs::write(disabled.with_extension("mbtiles.disabled"), b""));
    write_basemap(&valid, &[(6, 33, 43, b"valid")]);

    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(
        basemaps.files.keys().collect::<Vec<_>>(),
        [&broken, &disabled, &valid]
    );
    std::assert_matches!(&basemaps.files[&broken], BasemapSource::Unavailable(_));
    std::assert_matches!(&basemaps.files[&disabled], BasemapSource::Disabled);
    std::assert_matches!(&basemaps.files[&valid], BasemapSource::Active(_));
    assert_eq!(basemaps.resource_response("6/33/20.pbf").body(), b"valid");
    assert!(logs_contain("broken.mbtiles"));
    assert!(!logs_contain("disabled.mbtiles"));
}

#[test]
fn disabled_markers_persist_priority_across_reloads_and_new_files_start_enabled() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("a.mbtiles");
    let second = directory.path().join("b.mbtiles");
    write_basemap(&first, &[(6, 33, 43, b"first")]);
    write_basemap(&second, &[(6, 33, 43, b"second")]);
    assert_ok!(fs::write(first.with_extension("mbtiles.disabled"), b""));
    // A terrain marker with the same stem must not disable the basemap.
    assert_ok!(fs::write(second.with_extension("terrain.disabled"), b""));

    for _ in 0..2 {
        let basemaps = assert_ok!(Basemaps::load(directory.path()));
        assert_eq!(basemaps.resource_response("6/33/20.pbf").body(), b"second");
    }
    let added = directory.path().join("0.mbtiles");
    write_basemap(&added, &[(6, 33, 43, b"new")]);
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(basemaps.resource_response("6/33/20.pbf").body(), b"new");

    assert_ok!(fs::remove_file(first.with_extension("mbtiles.disabled")));
    assert_ok!(fs::write(added.with_extension("mbtiles.disabled"), b""));
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(basemaps.resource_response("6/33/20.pbf").body(), b"first");

    for path in [&first, &second] {
        assert_ok!(fs::write(path.with_extension("mbtiles.disabled"), b""));
    }
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    let response = basemaps.resource_response("6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.body().is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_disabled_marker_fails_the_scan() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("local.mbtiles");
    write_basemap(&path, &[]);
    let marker = path.with_extension("mbtiles.disabled");
    assert_ok!(std::os::unix::fs::symlink(&marker, &marker));

    claims::assert_err!(Basemaps::load(directory.path()).map(|_| ()));
}

#[test]
#[tracing_test::traced_test]
fn subscription_sends_the_inventory_through_ipc_and_can_be_closed() {
    use serde_json::{Value, json};
    let directory = tempfile::tempdir().unwrap();
    let active = directory.path().join("active.mbtiles");
    let disabled = directory.path().join("disabled.mbtiles");
    write_basemap(&active, &[]);
    assert_ok!(fs::write(&disabled, b"not sqlite"));
    assert_ok!(fs::write(disabled.with_extension("mbtiles.disabled"), b""));
    assert_ok!(fs::write(
        directory.path().join("invalid.mbtiles"),
        b"not sqlite"
    ));
    let basemaps = Arc::new(Mutex::new(assert_ok!(Basemaps::load(directory.path()))));
    let messages = Arc::new(Mutex::new(Vec::<Value>::new()));
    let received = messages.clone();
    let app = tauri::test::mock_builder()
        .manage(basemaps.clone())
        .channel_interceptor(move |_, _, _, body| {
            received
                .lock()
                .unwrap()
                .push(body.clone().deserialize().unwrap());
            true
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscribe_basemaps,
            commands::unsubscribe_basemaps
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = |command: &str, body| {
        let request = tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        };
        tauri::test::get_ipc_response(&window, request)
            .map(|response| response.deserialize::<Value>().unwrap())
    };
    for id in [42, 43] {
        let body = json!({"channel":format!("__CHANNEL__:{id}")});
        assert_eq!(assert_ok!(invoke("subscribe_basemaps", body)), Value::Null);
    }
    let messages = messages.lock().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], messages[1]);
    insta::assert_json_snapshot!(messages[0], @r#"
    {
      "generation": 0,
      "sources": [
        {
          "sourceName": "active.mbtiles",
          "type": "active"
        },
        {
          "sourceName": "disabled.mbtiles",
          "type": "disabled"
        },
        {
          "sourceName": "invalid.mbtiles",
          "type": "unavailable"
        }
      ]
    }
    "#);
    for _ in 0..2 {
        let body = json!({"channelId":42});
        assert_eq!(
            assert_ok!(invoke("unsubscribe_basemaps", body)),
            Value::Null
        );
    }
    let basemaps = basemaps.lock().unwrap();
    assert_eq!(
        basemaps.subscribers.keys().copied().collect::<Vec<_>>(),
        [43]
    );
    assert!(logs_contain("Could not open offline basemap"));
}

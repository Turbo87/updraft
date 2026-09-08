use super::*;
use crate::enroute::{
    commands::DownloadCommands, parse_catalog, queue::DownloadState,
    storage::remove_partial_downloads,
};
use claims::{assert_err, assert_ok, assert_some};
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
fn activation_persists_and_invalidates_tiles_without_changing_priority() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let first = directory.path().join("enroute/Europe/a.mbtiles");
    write_basemap(&first, &[(6, 33, 43, b"first")]);
    let second = directory.path().join("enroute/Europe/b.mbtiles");
    write_basemap(&second, &[(6, 33, 43, b"second")]);
    let mut basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_ok!(basemaps.set_enabled("enroute/Europe/a.mbtiles", false));
    let stale = basemaps.resource_response("0/6/33/20.pbf");
    assert_eq!(stale.status(), StatusCode::NO_CONTENT);
    let tile = basemaps.resource_response("1/6/33/20.pbf");
    assert_eq!(tile.body(), b"second");
    let restarted = assert_ok!(Basemaps::load(directory.path()));
    let tile = restarted.resource_response("0/6/33/20.pbf");
    assert_eq!(tile.body(), b"second");
    assert_ok!(basemaps.set_enabled("enroute/Europe/a.mbtiles", true));
    assert!(!first.with_extension("mbtiles.disabled").exists());
    assert_eq!(basemaps.resource_response("2/6/33/20.pbf").body(), b"first");
    assert_ok!(basemaps.set_enabled("enroute/Europe/a.mbtiles", true));
    assert_eq!(basemaps.generation, 3);
    let channel = Channel::new(|_| Err(std::io::Error::other("closed channel").into()));
    basemaps.subscribers.insert(channel.id(), channel);
    assert_ok!(basemaps.set_enabled("enroute/Europe/a.mbtiles", false));
    assert!(basemaps.subscribers.is_empty());
    assert!(first.with_extension("mbtiles.disabled").exists());
    let tile = basemaps.resource_response("4/6/33/20.pbf");
    assert_eq!(tile.body(), b"second");
}

#[test]
fn marker_failures_leave_activation_and_generation_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path_id = "enroute/Europe/local.mbtiles";
    let path = directory.path().join(path_id);
    write_basemap(&path, &[(6, 33, 43, b"tile")]);
    let mut basemaps = assert_ok!(Basemaps::load(directory.path()));
    let marker = path.with_extension("mbtiles.disabled");
    assert_ok!(fs::create_dir(&marker));
    assert_err!(basemaps.set_enabled("enroute/Europe/local.mbtiles", false));
    assert_eq!(basemaps.generation, 0);
    assert_eq!(basemaps.resource_response("0/6/33/20.pbf").body(), b"tile");
    let mut disabled = assert_ok!(Basemaps::load(directory.path()));
    assert_err!(disabled.set_enabled("enroute/Europe/local.mbtiles", true));
    assert_eq!(disabled.generation, 0);
    std::assert_matches!(disabled.files[path_id], BasemapSource::Disabled);
    for name in [
        "enroute/Europe/../local.mbtiles",
        "enroute/Europe/missing.mbtiles",
        "local.terrain",
    ] {
        assert_err!(basemaps.set_enabled(name, false));
    }
}

#[test]
fn removal_clears_markers_and_falls_back_to_remaining_tiles() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let first = directory.path().join("enroute/Europe/a.mbtiles");
    let second = directory.path().join("enroute/Europe/b.mbtiles");
    write_basemap(&first, &[(6, 33, 43, b"first")]);
    write_basemap(&second, &[(6, 33, 43, b"second")]);
    let mut basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_err!(basemaps.remove("enroute/Europe/../a.mbtiles"));
    assert_ok!(basemaps.remove("enroute/Europe/a.mbtiles"));
    assert!(!first.exists());
    let tile = basemaps.resource_response("1/6/33/20.pbf");
    assert_eq!(tile.body(), b"second");
    let stale = basemaps.resource_response("0/6/33/20.pbf");
    assert_eq!(stale.status(), StatusCode::NO_CONTENT);
    assert_ok!(basemaps.set_enabled("enroute/Europe/b.mbtiles", false));
    assert_ok!(basemaps.remove("enroute/Europe/b.mbtiles"));
    assert!(!second.exists());
    assert!(!second.with_extension("mbtiles.disabled").exists());
    assert!(basemaps.files.is_empty());
    write_basemap(&second, &[(6, 33, 43, b"new")]);
    let restarted = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(restarted.resource_response("0/6/33/20.pbf").body(), b"new");
}

#[test]
fn removal_failures_can_be_retried_without_opening_disabled_files() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path_id = "enroute/Europe/local.mbtiles";
    let path = directory.path().join(path_id);
    let marker = path.with_extension("mbtiles.disabled");
    assert_ok!(fs::write(&path, b"not sqlite"));
    assert_ok!(fs::create_dir(&marker));
    let mut basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_ok!(fs::remove_file(&path));
    assert_ok!(fs::create_dir(&path));
    assert_err!(basemaps.remove("enroute/Europe/local.mbtiles"));
    assert!(path.exists());
    assert!(marker.exists());
    std::assert_matches!(basemaps.files[path_id], BasemapSource::Disabled);
    assert_ok!(fs::remove_dir(&path));
    assert_ok!(fs::write(&path, b"not sqlite"));
    assert_err!(basemaps.remove("enroute/Europe/local.mbtiles"));
    assert!(!path.exists());
    std::assert_matches!(basemaps.files[path_id], BasemapSource::Disabled);
    assert_ok!(fs::remove_dir(&marker));
    assert_ok!(fs::write(&marker, b""));
    assert_ok!(basemaps.remove("enroute/Europe/local.mbtiles"));
    assert!(!marker.exists());
    assert!(basemaps.files.is_empty());
}

#[test]
fn marker_cleanup_failure_stops_serving_deleted_tiles_and_allows_retry() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path_id = "enroute/Europe/local.mbtiles";
    let path = directory.path().join(path_id);
    let marker = path.with_extension("mbtiles.disabled");
    write_basemap(&path, &[(6, 33, 43, b"tile")]);
    let mut basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_ok!(fs::create_dir(&marker));
    assert_err!(basemaps.remove("enroute/Europe/local.mbtiles"));
    assert!(!path.exists());
    std::assert_matches!(basemaps.files[path_id], BasemapSource::Unavailable(_));
    let tile = basemaps.resource_response("1/6/33/20.pbf");
    assert_eq!(tile.status(), StatusCode::NO_CONTENT);
    assert_ok!(fs::remove_dir(&marker));
    assert_ok!(basemaps.remove("enroute/Europe/local.mbtiles"));
    assert!(basemaps.files.is_empty());
}

#[test]
#[tracing_test::traced_test]
fn enabling_invalid_files_retains_them_and_disabling_does_not_open_them() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path_id = "enroute/Europe/broken.mbtiles";
    let path = directory.path().join(path_id);
    assert_ok!(fs::write(&path, b"not sqlite"));
    assert_ok!(fs::write(path.with_extension("mbtiles.disabled"), b""));
    let mut basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_ok!(basemaps.set_enabled("enroute/Europe/broken.mbtiles", false));
    assert!(!logs_contain("Could not open offline basemap"));
    assert_ok!(basemaps.set_enabled("enroute/Europe/broken.mbtiles", true));
    std::assert_matches!(basemaps.files[path_id], BasemapSource::Unavailable(_));
    assert!(logs_contain("Could not open offline basemap"));
    assert_ok!(basemaps.set_enabled("enroute/Europe/broken.mbtiles", false));
    std::assert_matches!(basemaps.files[path_id], BasemapSource::Disabled);
}

#[test]
fn serves_the_first_tile_in_filename_order_with_xyz_coordinates() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let germany = directory.path().join("enroute/Europe/Germany.mbtiles");
    write_basemap(&germany, &[(6, 33, 43, b"germany")]);
    let france = directory.path().join("enroute/Europe/France.mbtiles");
    write_basemap(&france, &[(6, 33, 43, b"france")]);

    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    let response = basemaps.resource_response("0/6/33/20.pbf");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = &response.headers()[header::CONTENT_TYPE];
    assert_eq!(content_type, "application/vnd.mapbox-vector-tile");
    assert_eq!(response.body(), b"france");
}

#[test]
fn serves_tiles_without_zoom_or_attribution_metadata() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path = directory.path().join("enroute/Europe/minimal.mbtiles");
    write_basemap(&path, &[(6, 33, 43, b"tile")]);
    Connection::open(path)
        .unwrap()
        .execute("DELETE FROM metadata WHERE name != 'format'", [])
        .unwrap();

    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(basemaps.resource_response("0/6/33/20.pbf").body(), b"tile");
}

#[test]
#[tracing_test::traced_test]
fn skips_invalid_files_and_continues_with_valid_basemaps() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    std::fs::write(
        directory.path().join("enroute/Europe/broken.mbtiles"),
        b"not sqlite",
    )
    .unwrap();
    let raster_format = "UPDATE metadata SET value = 'png' WHERE name = 'format'";
    for (name, update) in [("raster", raster_format), ("schema", "DROP TABLE tiles")] {
        let path = directory
            .path()
            .join(format!("enroute/Europe/{name}.mbtiles"));
        write_basemap(&path, &[]);
        Connection::open(path)
            .unwrap()
            .execute_batch(update)
            .unwrap();
    }
    let valid = directory.path().join("enroute/Europe/valid.mbtiles");
    write_basemap(&valid, &[(6, 33, 43, b"valid")]);
    let ignored = directory.path().join("ignored.sqlite");
    write_basemap(&ignored, &[(6, 33, 43, b"ignored")]);

    let basemaps = assert_ok!(Basemaps::load(directory.path()));

    assert_eq!(basemaps.resource_response("0/6/33/20.pbf").body(), b"valid");
    for name in ["broken.mbtiles", "raster.mbtiles", "schema.mbtiles"] {
        assert!(logs_contain(name));
    }
    assert!(logs_contain("Could not open offline basemap"));
}

#[test]
fn missing_directory_has_no_tiles() {
    let directory = tempfile::tempdir().unwrap();
    let basemaps = assert_ok!(Basemaps::load(directory.path()));

    let response = basemaps.resource_response("0/6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.body().is_empty());
}

#[test]
fn looks_in_later_files_and_serves_both_antimeridian_columns() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let east = directory.path().join("enroute/Europe/east.mbtiles");
    write_basemap(&east, &[(6, 63, 63, b"east")]);
    let west = directory.path().join("enroute/Europe/west.mbtiles");
    write_basemap(&west, &[(6, 0, 0, b"west")]);
    for name in ["east", "west"] {
        let path = directory
            .path()
            .join(format!("enroute/Europe/{name}.mbtiles"));
        let insert_bounds = "INSERT INTO metadata VALUES ('bounds', '170,-20,-170,20')";
        Connection::open(path)
            .unwrap()
            .execute(insert_bounds, [])
            .unwrap();
    }
    let basemaps = assert_ok!(Basemaps::load(directory.path()));

    assert_eq!(basemaps.resource_response("0/6/63/0.pbf").body(), b"east");
    assert_eq!(basemaps.resource_response("0/6/0/63.pbf").body(), b"west");
    let response = basemaps.resource_response("0/6/1/1.pbf");
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
        let response = basemaps.resource_response(&format!("0/{path}"));
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
    }
}

#[test]
#[tracing_test::traced_test]
fn reports_database_and_gzip_failures_as_errors_instead_of_missing_tiles() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path = directory.path().join("enroute/Europe/corrupt.mbtiles");
    write_basemap(&path, &[(6, 33, 43, b"tile")]);
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute("UPDATE tiles SET tile_data = ?1", [b"not gzip".as_slice()])
        .unwrap();

    let response = basemaps.resource_response("0/6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    connection.execute("DROP TABLE tiles", []).unwrap();
    let response = basemaps.resource_response("0/6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(logs_contain("Could not read offline basemap tile"));
}

#[test]
#[tracing_test::traced_test]
fn inventory_retains_invalid_files_and_does_not_open_disabled_files() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let broken_id = "enroute/Europe/broken.mbtiles";
    let broken = directory.path().join(broken_id);
    let disabled_id = "enroute/Europe/disabled.mbtiles";
    let disabled = directory.path().join(disabled_id);
    let valid_id = "enroute/Europe/valid.mbtiles";
    let valid = directory.path().join(valid_id);
    assert_ok!(fs::write(&broken, b"not sqlite"));
    assert_ok!(fs::write(&disabled, b"also not sqlite"));
    assert_ok!(fs::write(disabled.with_extension("mbtiles.disabled"), b""));
    write_basemap(&valid, &[(6, 33, 43, b"valid")]);

    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(
        basemaps.files.keys().collect::<Vec<_>>(),
        [broken_id, disabled_id, valid_id]
    );
    std::assert_matches!(&basemaps.files[broken_id], BasemapSource::Unavailable(_));
    std::assert_matches!(&basemaps.files[disabled_id], BasemapSource::Disabled);
    std::assert_matches!(&basemaps.files[valid_id], BasemapSource::Active(_));
    assert_eq!(basemaps.resource_response("0/6/33/20.pbf").body(), b"valid");
    assert!(logs_contain("broken.mbtiles"));
    assert!(!logs_contain("disabled.mbtiles"));
}

#[test]
fn disabled_markers_persist_priority_across_reloads_and_new_files_start_enabled() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let first = directory.path().join("enroute/Europe/a.mbtiles");
    let second = directory.path().join("enroute/Europe/b.mbtiles");
    write_basemap(&first, &[(6, 33, 43, b"first")]);
    write_basemap(&second, &[(6, 33, 43, b"second")]);
    assert_ok!(fs::write(first.with_extension("mbtiles.disabled"), b""));
    // A terrain marker with the same stem must not disable the basemap.
    assert_ok!(fs::write(second.with_extension("terrain.disabled"), b""));

    for _ in 0..2 {
        let basemaps = assert_ok!(Basemaps::load(directory.path()));
        let tile = basemaps.resource_response("0/6/33/20.pbf");
        assert_eq!(tile.body(), b"second");
    }
    let added = directory.path().join("enroute/Europe/0.mbtiles");
    write_basemap(&added, &[(6, 33, 43, b"new")]);
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(basemaps.resource_response("0/6/33/20.pbf").body(), b"new");

    assert_ok!(fs::remove_file(first.with_extension("mbtiles.disabled")));
    assert_ok!(fs::write(added.with_extension("mbtiles.disabled"), b""));
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    assert_eq!(basemaps.resource_response("0/6/33/20.pbf").body(), b"first");

    for path in [&first, &second] {
        assert_ok!(fs::write(path.with_extension("mbtiles.disabled"), b""));
    }
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    let response = basemaps.resource_response("0/6/33/20.pbf");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.body().is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_disabled_marker_fails_the_scan() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path = directory.path().join("enroute/Europe/local.mbtiles");
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
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let active = directory.path().join("enroute/Europe/active.mbtiles");
    let disabled = directory.path().join("enroute/Europe/disabled.mbtiles");
    write_basemap(&active, &[]);
    assert_ok!(fs::write(&disabled, b"not sqlite"));
    assert_ok!(fs::write(disabled.with_extension("mbtiles.disabled"), b""));
    assert_ok!(fs::write(
        directory.path().join("enroute/Europe/invalid.mbtiles"),
        b"not sqlite"
    ));
    let basemaps = Arc::new(Mutex::new(assert_ok!(Basemaps::load(directory.path()))));
    let messages = Arc::new(Mutex::new(Vec::<Value>::new()));
    let received = messages.clone();
    let downloads = DownloadCommands::default();
    let queue = downloads.queue.clone();
    let app = tauri::test::mock_builder()
        .manage(basemaps.clone())
        .manage(downloads)
        .channel_interceptor(move |_, _, _, body| {
            received
                .lock()
                .unwrap()
                .push(body.clone().deserialize().unwrap());
            true
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscribe_basemaps,
            commands::unsubscribe_basemaps,
            commands::set_basemap_enabled,
            commands::remove_basemap
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
    let publications = messages.clone();
    let messages = messages.lock().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], messages[1]);
    insta::assert_json_snapshot!(messages[0], @r#"
    {
      "generation": 0,
      "sources": [
        {
          "sourceName": "enroute/Europe/active.mbtiles",
          "type": "active"
        },
        {
          "sourceName": "enroute/Europe/disabled.mbtiles",
          "type": "disabled"
        },
        {
          "sourceName": "enroute/Europe/invalid.mbtiles",
          "type": "unavailable"
        }
      ]
    }
    "#);
    drop(messages);
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
    drop(basemaps);
    let body = json!({"sourceName":"enroute/Europe/active.mbtiles", "enabled":false});
    assert_eq!(assert_ok!(invoke("set_basemap_enabled", body)), Value::Null);
    let messages = publications.lock().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[2]["generation"], 1);
    assert_eq!(
        messages[2]["sources"][0],
        json!({"sourceName":"enroute/Europe/active.mbtiles", "type":"disabled"})
    );
    drop(messages);
    let body = json!({"sourceName":"enroute/Europe/missing.mbtiles", "enabled":true});
    assert_eq!(
        invoke("set_basemap_enabled", body),
        Err(json!("Could not change basemap activation"))
    );
    let catalog = br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
    let mut entry = assert_ok!(parse_catalog(catalog)).remove(0);
    entry.path = "Europe/active.mbtiles";
    let attempt = {
        let mut queue = queue.lock().unwrap();
        queue.enqueue(entry);
        assert_some!(queue.start_next())
    };
    let download = assert_ok!(BasemapDownload::new(directory.path(), &attempt));
    let body = json!({"sourceName":"enroute/Europe/active.mbtiles"});
    assert_eq!(assert_ok!(invoke("remove_basemap", body)), Value::Null);
    assert!(!queue.lock().unwrap().is_active(&attempt));
    use tauri::Manager;
    let basemaps = app.state::<Arc<Mutex<Basemaps>>>();
    app.state::<DownloadCommands>()
        .install_download(basemaps.inner(), &attempt, download);
    assert!(!active.exists());
    let messages = publications.lock().unwrap();
    assert_eq!(messages.len(), 4);
    insta::assert_json_snapshot!(messages[3], @r#"
    {
      "generation": 2,
      "sources": [
        {
          "sourceName": "enroute/Europe/disabled.mbtiles",
          "type": "disabled"
        },
        {
          "sourceName": "enroute/Europe/invalid.mbtiles",
          "type": "unavailable"
        }
      ]
    }
    "#);
    assert!(logs_contain("Could not open offline basemap"));
}

#[test]
fn managed_identities_keep_same_name_files_independent() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let first = "enroute/Asia/Georgia.mbtiles";
    let second = "enroute/North America/United States/Georgia.mbtiles";
    for id in [first, second] {
        let path = root.join(id);
        assert_ok!(fs::create_dir_all(path.parent().unwrap()));
        write_basemap(&path, &[(6, 33, 43, id.as_bytes())]);
    }
    let legacy = root.join("enroute/Georgia.mbtiles");
    write_basemap(&legacy, &[(6, 33, 43, b"legacy")]);
    let mut basemaps = assert_ok!(Basemaps::load(root));
    assert_eq!(
        basemaps.resource_response("0/6/33/20.pbf").body(),
        first.as_bytes()
    );
    assert_err!(basemaps.set_enabled("Georgia.mbtiles", false));
    assert_ok!(basemaps.set_enabled(first, false));
    let restarted = assert_ok!(Basemaps::load(root));
    assert_eq!(
        restarted.resource_response("0/6/33/20.pbf").body(),
        second.as_bytes()
    );
    assert_ok!(basemaps.remove(first));
    assert!(!root.join(first).exists());
    assert!(root.join(second).exists());
    assert!(legacy.exists());
}

fn basemap_download(directory: &Path, bytes: &[u8]) -> BasemapDownload {
    let json = br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
    let entry = assert_ok!(parse_catalog(json)).remove(0);
    let mut download = assert_ok!(BasemapDownload::new(directory, &entry));
    assert_ok!(download.file_mut().write_all(bytes));
    download
}

#[test]
#[tracing_test::traced_test]
fn downloaded_basemaps_replace_tiles_and_preserve_activation() {
    let name = "enroute/Europe/Germany.mbtiles";
    for installed in [false, true] {
        for disabled in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(name);
            assert_ok!(fs::create_dir_all(path.parent().unwrap()));
            if installed {
                write_basemap(&path, &[(6, 33, 43, b"old")]);
            }
            if disabled {
                assert_ok!(fs::write(path.with_extension("mbtiles.disabled"), b""));
            }
            let mut basemaps = assert_ok!(Basemaps::load(directory.path()));
            let replacement = directory.path().join("replacement.mbtiles");
            write_basemap(&replacement, &[(6, 33, 43, b"new")]);
            let download = basemap_download(directory.path(), &assert_ok!(fs::read(replacement)));
            assert_ok!(basemaps.install_download(name, download));
            assert_eq!(basemaps.generation, 1);
            let disabled = installed && disabled;
            assert_eq!(
                matches!(basemaps.files[name], BasemapSource::Disabled),
                disabled
            );
            assert_eq!(path.with_extension("mbtiles.disabled").exists(), disabled);
            let tile = basemaps.resource_response("1/6/33/20.pbf");
            let expected: &[u8] = if disabled { b"" } else { b"new" };
            assert_eq!(tile.body().as_slice(), expected);
            let download = basemap_download(directory.path(), b"invalid");
            assert_ok!(basemaps.install_download(name, download));
            assert_eq!(assert_ok!(fs::read(&path)), b"invalid");
            assert_eq!(basemaps.generation, 2);
            assert_eq!(
                matches!(basemaps.files[name], BasemapSource::Unavailable(_)),
                !disabled
            );
        }
    }
    assert!(logs_contain("Could not open downloaded basemap"));
}

#[test]
#[tracing_test::traced_test]
fn installation_finishes_the_queue_and_respects_cancellation() {
    for (cancelled, failed) in [(false, false), (false, true), (true, false)] {
        let directory = tempfile::tempdir().unwrap();
        let name = "enroute/Europe/Germany.mbtiles";
        let path = directory.path().join(name);
        assert_ok!(fs::create_dir_all(path.parent().unwrap()));
        write_basemap(&path, &[(6, 33, 43, b"old")]);
        let download = basemap_download(directory.path(), &assert_ok!(fs::read(&path)));
        if failed {
            assert_ok!(remove_partial_downloads(directory.path()));
        }
        let basemaps = Mutex::new(assert_ok!(Basemaps::load(directory.path())));
        let state = DownloadCommands::default();
        let json = br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
        let entry = assert_ok!(parse_catalog(json)).remove(0);
        let attempt = {
            let mut queue = state.queue.lock().unwrap();
            queue.enqueue(entry);
            let attempt = assert_some!(queue.start_next());
            if cancelled {
                queue.cancel(attempt.path);
            }
            attempt
        };
        state.install_download(&basemaps, &attempt, download);
        let status = state.queue.lock().unwrap().subscribe();
        assert_eq!(status.borrow().len(), usize::from(failed));
        if failed {
            std::assert_matches!(status.borrow()[0].state, DownloadState::Failed);
        }
        let basemaps = basemaps.lock().unwrap();
        assert_eq!(basemaps.generation, u64::from(!cancelled));
        let resource = format!("{}/6/33/20.pbf", basemaps.generation);
        assert_eq!(basemaps.resource_response(&resource).body(), b"old");
        assert_eq!(assert_ok!(fs::read_dir(path.parent().unwrap())).count(), 1);
    }
    assert!(logs_contain("Could not install downloaded basemap"));
}

#[test]
fn available_updates_include_active_and_disabled_files_only_when_newer() {
    use std::fs::{FileTimes, OpenOptions};
    use std::time::Duration;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("enroute/Europe");
    assert_ok!(fs::create_dir_all(&root));
    let entries = assert_ok!(parse_catalog(
        br#"{"maps":[
        {"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"},
        {"path":"Europe/France.mbtiles","size":10,"time":"20260908"},
        {"path":"Europe/Malta.mbtiles","size":10,"time":"20260908"}
    ]}"#
    ));
    let publication: std::time::SystemTime = time::macros::datetime!(2026-09-08 0:00 UTC).into();
    let germany = root.join("Germany.mbtiles");
    let france = root.join("France.mbtiles");
    write_basemap(&germany, &[]);
    assert_ok!(fs::write(&france, b"disabled, not a database"));
    assert_ok!(fs::write(france.with_extension("mbtiles.disabled"), b""));
    for path in [&germany, &france] {
        let file = assert_ok!(OpenOptions::new().write(true).open(path));
        assert_ok!(
            file.set_times(FileTimes::new().set_modified(publication - Duration::from_secs(1)))
        );
    }
    let basemaps = assert_ok!(Basemaps::load(directory.path()));
    insta::assert_json_snapshot!(assert_ok!(basemaps.available_updates(&entries)), @r#"
    [
      "Europe/France.mbtiles",
      "Europe/Germany.mbtiles"
    ]
    "#);
    for offset in [0, 1] {
        for path in [&germany, &france] {
            let file = assert_ok!(OpenOptions::new().write(true).open(path));
            assert_ok!(file.set_times(
                FileTimes::new().set_modified(publication + Duration::from_secs(offset))
            ));
        }
        assert!(assert_ok!(basemaps.available_updates(&entries)).is_empty());
    }
    assert_ok!(fs::remove_file(germany));
    assert_err!(basemaps.available_updates(&entries));
}

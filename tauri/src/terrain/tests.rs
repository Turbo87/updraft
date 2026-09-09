use super::*;
use crate::basemap::Basemaps;
use crate::enroute::{CatalogEntry, Continent, commands::DownloadCommands, download::DownloadFile};
use crate::enroute::{queue::DownloadState, storage::remove_partial_downloads};
use claims::{assert_err, assert_ok, assert_some};
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
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let france = tagged_webp(b"france");
    let germany = tagged_webp(b"germany");
    let west = tagged_webp(b"west");
    let east = tagged_webp(b"east");
    write_terrain(
        &directory.path().join("enroute/Europe/Germany.terrain"),
        &[(7, 66, 87, &germany), (10, 0, 0, &west)],
    );
    write_terrain(
        &directory.path().join("enroute/Europe/France.terrain"),
        &[(7, 66, 87, &france), (10, 1023, 1023, &east)],
    );
    write_terrain(
        &directory.path().join("Basemap.mbtiles"),
        &[(7, 66, 87, b"ignored")],
    );
    let terrain = assert_ok!(Terrain::load(directory.path()));

    let response = terrain.resource_response("0/7/66/40.webp");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/webp");
    assert_eq!(response.body(), &france);
    assert_eq!(terrain.resource_response("0/10/0/1023.webp").body(), &west);
    assert_eq!(terrain.resource_response("0/10/1023/0.webp").body(), &east);
    assert_eq!(
        terrain.resource_response("0/7/0/0.webp").status(),
        StatusCode::NOT_FOUND
    );
}

#[test]
#[tracing_test::traced_test]
fn retains_invalid_files_without_contributing_tiles_or_metadata() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    std::fs::write(
        directory.path().join("enroute/Europe/broken.terrain"),
        b"not sqlite",
    )
    .unwrap();
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
        (
            "attribution",
            "INSERT INTO metadata VALUES ('attribution', NULL)",
        ),
    ] {
        let path_id = format!("enroute/Europe/{name}.terrain");
        let path = directory.path().join(&path_id);
        write_terrain(&path, &[]);
        Connection::open(path).unwrap().execute_batch(sql).unwrap();
    }
    let tile = webp_header(256, 256);
    write_terrain(
        &directory.path().join("enroute/Europe/valid.terrain"),
        &[(7, 66, 87, &tile)],
    );
    let terrain = assert_ok!(Terrain::load(directory.path()));
    assert_eq!(terrain.resource_response("0/7/66/40.webp").body(), &tile);
    assert_eq!(
        terrain.resource_response("0/metadata.json").status(),
        StatusCode::OK
    );
    for name in [
        "broken",
        "format",
        "encoding",
        "missing",
        "schema",
        "attribution",
    ] {
        let path_id = format!("enroute/Europe/{name}.terrain");
        std::assert_matches!(
            terrain.files[path_id.as_str()],
            TerrainSource::Unavailable(_)
        );
        assert!(logs_contain(&format!("{name}.terrain")));
    }
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
fn missing_directory_is_empty_but_scan_failure_is_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("enroute");
    let terrain = assert_ok!(Terrain::load(directory.path()));
    let response = terrain.resource_response("0/7/66/40.webp");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.body().is_empty());
    std::fs::write(&path, b"not a directory").unwrap();
    assert_err!(Terrain::load(directory.path()).map(|_| ()));
}

#[test]
fn rejects_invalid_requests() {
    let terrain = Terrain::default();
    for path in [
        "0/32/0/0.webp",
        "0/7/128/0.webp",
        "0/7/0/128.webp",
        "0/7/-1/0.webp",
        "0/7/0.webp",
        "0/7/0/0/0.webp",
        "0/a/0/0.webp",
        "0/7/0/0.pbf",
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
fn reports_tile_read_failures_and_retains_validated_metadata() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path = directory.path().join("enroute/Europe/Germany.terrain");
    write_terrain(&path, &[]);
    let terrain = assert_ok!(Terrain::load(directory.path()));
    let metadata = assert_ok!(terrain.metadata());
    let connection = Connection::open(path).unwrap();
    connection.execute("DROP TABLE tiles", []).unwrap();
    assert_eq!(
        terrain.resource_response("0/7/66/40.webp").status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(logs_contain("Could not read offline terrain resource"));
    connection.execute("DROP TABLE metadata", []).unwrap();
    let response = terrain.resource_response("0/metadata.json");
    assert_eq!(response.body(), &metadata);
}

#[test]
fn serves_tilejson_with_combined_attributions_without_duplicates_or_placeholders() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let mut extended_header = b"RIFF\x16\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
    extended_header.extend_from_slice(&511_u32.to_le_bytes()[..3]);
    extended_header.extend_from_slice(&511_u32.to_le_bytes()[..3]);
    for (country, minzoom, maxzoom, tile) in [
        ("Germany", 5, 12, webp_header(512, 512)),
        ("France", 4, 11, extended_header),
    ] {
        let path = directory
            .path()
            .join(format!("enroute/Europe/{country}.terrain"));
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
    let response = terrain.resource_response("0/metadata.json");
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
        "updraft://localhost/terrain/0/{z}/{x}/{y}.webp"
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
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    write_terrain(&directory.path().join("enroute/Europe/empty.terrain"), &[]);
    let terrain = assert_ok!(Terrain::load(directory.path()));
    let response = terrain.resource_response("0/metadata.json");
    assert_eq!(response.status(), StatusCode::OK);
    let metadata: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    insta::assert_json_snapshot!(metadata, @r#"
    {
      "attribution": "",
      "encoding": "terrarium",
      "tilejson": "3.0.0",
      "tiles": [
        "updraft://localhost/terrain/0/{z}/{x}/{y}.webp"
      ]
    }
    "#);
    let empty = Terrain::default().resource_response("0/metadata.json");
    assert_eq!(response.body(), empty.body());
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
        fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
        write_terrain(
            &directory.path().join("enroute/Europe/France.terrain"),
            &[(7, 0, 0, &webp_header(256, 256))],
        );
        write_terrain(
            &directory.path().join("enroute/Europe/Germany.terrain"),
            &[(zoom, 1, 0, &tile)],
        );
        let terrain = assert_ok!(Terrain::load(directory.path()));
        let rejected_id = "enroute/Europe/Germany.terrain";
        std::assert_matches!(terrain.files[rejected_id], TerrainSource::Unavailable(_));
        assert_eq!(
            terrain.resource_response("0/metadata.json").status(),
            StatusCode::OK
        );
        if zoom < 32 {
            let path = format!("0/{zoom}/1/{}.webp", (1_u32 << zoom) - 1);
            assert_eq!(
                terrain.resource_response(&path).status(),
                StatusCode::NOT_FOUND
            );
        }
    }
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
#[tracing_test::traced_test]
fn inventory_rechecks_compatibility_and_excludes_disabled_files() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let empty_id = "enroute/Europe/00-empty.terrain";
    let empty = directory.path().join(empty_id);
    write_terrain(&empty, &[]);
    let invalid_id = "enroute/Europe/01-invalid.terrain";
    let invalid = directory.path().join(invalid_id);
    assert_ok!(fs::write(&invalid, b"not sqlite"));
    for (name, size, zoom) in [("a", 256, 6), ("b", 512, 8), ("c", 256, 7)] {
        let path = directory
            .path()
            .join(format!("enroute/Europe/{name}.terrain"));
        write_terrain(&path, &[(zoom, 0, 0, &webp_header(size, size))]);
        assert_ok!(
            Connection::open(path)
                .unwrap()
                .execute("INSERT INTO metadata VALUES ('attribution', ?1)", [name])
        );
    }
    let first_id = "enroute/Europe/a.terrain";
    let first = directory.path().join(first_id);
    let second_id = "enroute/Europe/b.terrain";
    let third_id = "enroute/Europe/c.terrain";
    assert_ok!(fs::write(first.with_extension("mbtiles.disabled"), b""));
    let terrain = assert_ok!(Terrain::load(directory.path()));
    assert_eq!(
        terrain.files.keys().collect::<Vec<_>>(),
        [empty_id, invalid_id, first_id, second_id, third_id]
    );
    std::assert_matches!(terrain.files[invalid_id], TerrainSource::Unavailable(_));
    std::assert_matches!(terrain.files[first_id], TerrainSource::Active(_));
    std::assert_matches!(terrain.files[second_id], TerrainSource::Unavailable(_));
    assert_eq!(
        terrain.resource_response("0/7/0/127.webp").body(),
        &webp_header(256, 256)
    );
    assert_eq!(
        terrain.resource_response("0/8/0/255.webp").status(),
        StatusCode::NOT_FOUND
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&assert_ok!(terrain.metadata())).unwrap();
    insta::assert_json_snapshot!(metadata, @r#"
    {
      "attribution": "a<br>c",
      "encoding": "terrarium",
      "maxzoom": 7,
      "minzoom": 6,
      "tileSize": 256,
      "tilejson": "3.0.0",
      "tiles": [
        "updraft://localhost/terrain/0/{z}/{x}/{y}.webp"
      ]
    }
    "#);
    assert_ok!(fs::write(first.with_extension("terrain.disabled"), b""));
    assert_ok!(fs::write(&first, b"disabled files must not be opened"));
    for _ in 0..2 {
        let terrain = assert_ok!(Terrain::load(directory.path()));
        std::assert_matches!(terrain.files[first_id], TerrainSource::Disabled);
        std::assert_matches!(terrain.files[second_id], TerrainSource::Active(_));
        std::assert_matches!(terrain.files[third_id], TerrainSource::Unavailable(_));
        assert_eq!(
            terrain.resource_response("0/8/0/255.webp").body(),
            &webp_header(512, 512)
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&assert_ok!(terrain.metadata())).unwrap();
        assert_eq!(metadata["attribution"], "b");
        assert_eq!(metadata["tileSize"], 512);
    }
    let added_id = "enroute/Europe/0-new.terrain";
    let added = directory.path().join(added_id);
    write_terrain(&added, &[(4, 0, 0, &webp_header(128, 128))]);
    let terrain = assert_ok!(Terrain::load(directory.path()));
    std::assert_matches!(terrain.files[added_id], TerrainSource::Active(_));
    std::assert_matches!(terrain.files[second_id], TerrainSource::Unavailable(_));
    for id in terrain.files.keys() {
        let path = directory.path().join(id);
        assert_ok!(fs::write(path.with_extension("terrain.disabled"), b""));
    }
    let terrain = assert_ok!(Terrain::load(directory.path()));
    assert!(
        terrain
            .files
            .values()
            .all(|source| matches!(source, TerrainSource::Disabled))
    );
    assert_eq!(
        assert_ok!(terrain.metadata()),
        assert_ok!(Terrain::default().metadata())
    );
    assert!(logs_contain("01-invalid.terrain"));
    assert!(!logs_contain("a.terrain"));
}

fn tagged_webp(tag: &[u8]) -> Vec<u8> {
    let mut tile = webp_header(256, 256);
    tile.extend_from_slice(tag);
    tile
}

#[cfg(unix)]
#[test]
fn unreadable_disabled_marker_fails_the_scan() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path = directory.path().join("enroute/Europe/local.terrain");
    write_terrain(&path, &[]);
    let marker = path.with_extension("terrain.disabled");
    assert_ok!(std::os::unix::fs::symlink(&marker, &marker));
    assert_err!(Terrain::load(directory.path()).map(|_| ()));
}

#[test]
#[tracing_test::traced_test]
fn subscription_sends_the_inventory_through_ipc_and_can_be_closed() {
    use serde_json::{Value, json};
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    for (name, size) in [("active", 256), ("incompatible", 512)] {
        let path = directory
            .path()
            .join(format!("enroute/Europe/{name}.terrain"));
        write_terrain(&path, &[(7, 0, 0, &webp_header(size, size))]);
    }
    let disabled = directory.path().join("enroute/Europe/disabled.terrain");
    assert_ok!(fs::write(&disabled, b"not sqlite"));
    assert_ok!(fs::write(disabled.with_extension("terrain.disabled"), b""));
    assert_ok!(fs::write(
        directory.path().join("enroute/Europe/invalid.terrain"),
        b"not sqlite"
    ));
    let terrain = Arc::new(Mutex::new(assert_ok!(Terrain::load(directory.path()))));
    let messages = Arc::new(Mutex::new(Vec::<Value>::new()));
    let received = messages.clone();
    let app = tauri::test::mock_builder()
        .manage(terrain.clone())
        .manage(DownloadCommands::default())
        .channel_interceptor(move |_, _, _, body| {
            let status = body.clone().deserialize().unwrap();
            received.lock().unwrap().push(status);
            true
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscribe_terrain,
            commands::unsubscribe_terrain,
            commands::set_terrain_enabled,
            commands::remove_terrain,
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
        assert_eq!(assert_ok!(invoke("subscribe_terrain", body)), Value::Null);
    }
    let initial = messages.lock().unwrap();
    assert_eq!(initial.len(), 2);
    assert_eq!(initial[0], initial[1]);
    insta::assert_json_snapshot!(initial[0], @r#"
    {
      "generation": 0,
      "sources": [
        {
          "sourceName": "enroute/Europe/active.terrain",
          "type": "active"
        },
        {
          "sourceName": "enroute/Europe/disabled.terrain",
          "type": "disabled"
        },
        {
          "sourceName": "enroute/Europe/incompatible.terrain",
          "type": "unavailable"
        },
        {
          "sourceName": "enroute/Europe/invalid.terrain",
          "type": "unavailable"
        }
      ]
    }
    "#);
    drop(initial);
    for id in [42, 42] {
        let body = json!({"channelId":id});
        assert_eq!(assert_ok!(invoke("unsubscribe_terrain", body)), Value::Null);
        let terrain = terrain.lock().unwrap();
        assert_eq!(
            terrain.subscribers.keys().copied().collect::<Vec<_>>(),
            [43]
        );
    }
    let body = json!({"sourceName":"enroute/Europe/active.terrain", "enabled":false});
    assert_eq!(assert_ok!(invoke("set_terrain_enabled", body)), Value::Null);
    let delivered = messages.lock().unwrap();
    assert_eq!(delivered.len(), 3);
    insta::assert_json_snapshot!(delivered[2], @r#"
    {
      "generation": 1,
      "sources": [
        {
          "sourceName": "enroute/Europe/active.terrain",
          "type": "disabled"
        },
        {
          "sourceName": "enroute/Europe/disabled.terrain",
          "type": "disabled"
        },
        {
          "sourceName": "enroute/Europe/incompatible.terrain",
          "type": "active"
        },
        {
          "sourceName": "enroute/Europe/invalid.terrain",
          "type": "unavailable"
        }
      ]
    }
    "#);
    drop(delivered);
    let body = json!({"sourceName":"enroute/Europe/missing.terrain", "enabled":true});
    assert_eq!(
        invoke("set_terrain_enabled", body),
        Err(json!("Could not change terrain activation"))
    );
    assert_eq!(messages.lock().unwrap().len(), 3);
    use tauri::Manager;
    let downloads = app.state::<DownloadCommands>();
    let attempt = {
        let mut queue = downloads.queue.lock().unwrap();
        queue.enqueue(terrain_entry("Europe/disabled.terrain"));
        assert_some!(queue.start_next())
    };
    let download = assert_ok!(DownloadFile::new(directory.path(), &attempt));
    let body = json!({"sourceName":"enroute/Europe/disabled.terrain"});
    assert_eq!(assert_ok!(invoke("remove_terrain", body)), Value::Null);
    let basemaps = Mutex::new(Basemaps::default());
    downloads.install_download(&basemaps, &terrain, &attempt, download);
    assert!(!disabled.exists());
    assert!(!downloads.queue.lock().unwrap().has_active());
    let removed = messages.lock().unwrap();
    assert_eq!(removed.len(), 4);
    assert_eq!(removed[3]["generation"], 2);
    let mut expected = removed[2]["sources"].as_array().unwrap().clone();
    expected.retain(|source| source["sourceName"] != "enroute/Europe/disabled.terrain");
    assert_eq!(removed[3]["sources"], json!(expected));
    drop(removed);
    assert_eq!(
        invoke(
            "remove_terrain",
            json!({"sourceName":"enroute/Europe/missing.terrain"})
        ),
        Err(json!("Could not remove terrain file"))
    );
    assert_eq!(messages.lock().unwrap().len(), 4);
    assert_eq!(
        assert_ok!(invoke("unsubscribe_terrain", json!({"channelId":43}))),
        Value::Null
    );
    assert!(terrain.lock().unwrap().subscribers.is_empty());
    assert!(logs_contain("incompatible.terrain"));
    assert!(logs_contain("invalid.terrain"));
    assert!(!logs_contain("disabled.terrain"));
}

#[test]
fn serves_only_the_requested_terrain_generation() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path = directory.path().join("enroute/Europe/local.terrain");
    let tile = webp_header(256, 256);
    write_terrain(&path, &[(7, 66, 87, &tile)]);
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    terrain.generation = 7;
    for path in ["0/metadata.json", "0/7/66/40.webp", "8/metadata.json"] {
        let response = terrain.resource_response(path);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.body().is_empty());
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    for path in [
        "metadata.json",
        "7/66/40.webp",
        "invalid/metadata.json",
        "18446744073709551616/metadata.json",
    ] {
        assert_eq!(
            terrain.resource_response(path).status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(terrain.resource_response("7/7/66/40.webp").body(), &tile);
    let response = terrain.resource_response("7/metadata.json");
    assert_eq!(response.status(), StatusCode::OK);
    let metadata: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    insta::assert_json_snapshot!(metadata, @r#"
    {
      "attribution": "",
      "encoding": "terrarium",
      "maxzoom": 7,
      "minzoom": 7,
      "tileSize": 256,
      "tilejson": "3.0.0",
      "tiles": [
        "updraft://localhost/terrain/7/{z}/{x}/{y}.webp"
      ]
    }
    "#);
}

#[test]
#[tracing_test::traced_test]
fn activation_rechecks_compatibility_and_persists_without_opening_disabled_files() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let first_id = "enroute/Europe/a.terrain";
    let first = directory.path().join(first_id);
    let second_id = "enroute/Europe/b.terrain";
    let second = directory.path().join(second_id);
    write_terrain(&first, &[(7, 66, 87, &webp_header(256, 256))]);
    write_terrain(&second, &[(7, 66, 87, &webp_header(512, 512))]);
    for (path, credit) in [(&first, "a"), (&second, "b")] {
        assert_ok!(
            Connection::open(path)
                .unwrap()
                .execute("INSERT INTO metadata VALUES ('attribution', ?1)", [credit])
        );
    }
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    assert_ok!(terrain.set_enabled("enroute/Europe/a.terrain", false));
    assert_eq!(terrain.generation, 1);
    assert!(first.with_extension("terrain.disabled").is_file());
    std::assert_matches!(terrain.files[second_id], TerrainSource::Active(_));
    assert_eq!(
        terrain.resource_response("1/7/66/40.webp").body(),
        &webp_header(512, 512)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&assert_ok!(terrain.metadata())).unwrap();
    assert_eq!(metadata["tileSize"], 512);
    assert_eq!(metadata["attribution"], "b");
    assert_eq!(
        terrain.resource_response("0/metadata.json").status(),
        StatusCode::NOT_FOUND
    );
    let restarted = assert_ok!(Terrain::load(directory.path()));
    std::assert_matches!(restarted.files[first_id], TerrainSource::Disabled);
    std::assert_matches!(restarted.files[second_id], TerrainSource::Active(_));
    drop(restarted);
    assert_ok!(fs::write(&first, b"disabled files must not be opened"));
    assert_ok!(terrain.set_enabled("enroute/Europe/b.terrain", false));
    assert_eq!(
        terrain.resource_response("2/7/66/40.webp").status(),
        StatusCode::NOT_FOUND
    );
    let channel = Channel::new(|_| Err(std::io::Error::other("closed channel").into()));
    terrain.subscribers.insert(channel.id(), channel);
    assert_ok!(terrain.set_enabled("enroute/Europe/b.terrain", true));
    assert!(terrain.subscribers.is_empty());
    assert!(!logs_contain("a.terrain"));
    assert_ok!(terrain.set_enabled("enroute/Europe/a.terrain", true));
    std::assert_matches!(terrain.files[first_id], TerrainSource::Unavailable(_));
    std::assert_matches!(terrain.files[second_id], TerrainSource::Active(_));
    assert!(!first.with_extension("terrain.disabled").exists());
    assert_ok!(fs::remove_file(&first));
    write_terrain(&first, &[(7, 66, 87, &webp_header(256, 256))]);
    assert_ok!(terrain.set_enabled("enroute/Europe/a.terrain", true));
    std::assert_matches!(terrain.files[first_id], TerrainSource::Active(_));
    std::assert_matches!(terrain.files[second_id], TerrainSource::Unavailable(_));
    assert_eq!(
        terrain.resource_response("5/7/66/40.webp").body(),
        &webp_header(256, 256)
    );
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
fn activation_marker_failures_preserve_inventory_and_reject_unknown_names() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path_id = "enroute/Europe/local.terrain";
    let path = directory.path().join(path_id);
    write_terrain(&path, &[]);
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    let marker = path.with_extension("terrain.disabled");
    assert_ok!(fs::create_dir(&marker));
    for enabled in [false, true] {
        assert_err!(terrain.set_enabled("enroute/Europe/local.terrain", enabled));
        std::assert_matches!(terrain.files[path_id], TerrainSource::Active(_));
        assert_eq!(terrain.generation, 0);
    }
    assert_ok!(fs::remove_dir(&marker));
    for name in [
        "enroute/Europe/missing.terrain",
        "enroute/Europe/../local.terrain",
        "enroute/Europe/nested/local.terrain",
    ] {
        assert_err!(terrain.set_enabled(name, false));
        assert_eq!(terrain.generation, 0);
    }
    assert_ok!(terrain.set_enabled("enroute/Europe/local.terrain", false));
    assert_ok!(terrain.set_enabled("enroute/Europe/local.terrain", false));
    assert_eq!(terrain.generation, 2);
}

#[test]
#[tracing_test::traced_test]
fn removal_rechecks_compatibility_and_clears_disabled_markers() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let first_id = "enroute/Europe/a.terrain";
    let first = directory.path().join(first_id);
    let second_id = "enroute/Europe/b.terrain";
    let second = directory.path().join(second_id);
    write_terrain(&first, &[(7, 66, 87, &webp_header(256, 256))]);
    write_terrain(&second, &[(7, 66, 87, &webp_header(512, 512))]);
    for (path, credit) in [(&first, "a"), (&second, "b")] {
        let connection = Connection::open(path).unwrap();
        assert_ok!(connection.execute("INSERT INTO metadata VALUES ('attribution', ?1)", [credit]));
    }
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    for name in [
        "enroute/Europe/missing.terrain",
        "enroute/Europe/../a.terrain",
    ] {
        assert_err!(terrain.remove(name));
        assert_eq!(terrain.generation, 0);
    }
    assert_ok!(terrain.remove("enroute/Europe/a.terrain"));
    assert!(!first.exists());
    assert!(!terrain.files.contains_key(first_id));
    std::assert_matches!(terrain.files[second_id], TerrainSource::Active(_));
    assert_eq!(
        terrain.resource_response("1/7/66/40.webp").body(),
        &webp_header(512, 512)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&assert_ok!(terrain.metadata())).unwrap();
    assert_eq!(metadata["tileSize"], 512);
    assert_eq!(metadata["attribution"], "b");
    assert_ok!(terrain.set_enabled("enroute/Europe/b.terrain", false));
    assert_ok!(fs::write(&second, b"disabled file must stay unopened"));
    assert_ok!(terrain.remove("enroute/Europe/b.terrain"));
    assert!(!second.exists());
    assert!(!second.with_extension("terrain.disabled").exists());
    assert!(terrain.files.is_empty());
    assert!(assert_ok!(Terrain::load(directory.path())).files.is_empty());
    assert_eq!(
        terrain.resource_response("3/7/66/40.webp").status(),
        StatusCode::NOT_FOUND
    );
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
#[tracing_test::traced_test]
fn removal_failures_publish_current_inventory_and_allow_retry() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("enroute/Europe")).unwrap();
    let path_id = "enroute/Europe/local.terrain";
    let path = directory.path().join(path_id);
    write_terrain(&path, &[]);
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    assert_ok!(fs::remove_file(&path));
    assert_ok!(fs::create_dir(&path));
    assert_err!(terrain.remove("enroute/Europe/local.terrain"));
    assert_eq!(terrain.generation, 1);
    std::assert_matches!(terrain.files[path_id], TerrainSource::Unavailable(_));
    assert_ok!(fs::remove_dir(&path));
    write_terrain(&path, &[]);
    let marker = path.with_extension("terrain.disabled");
    assert_ok!(fs::create_dir(&marker));
    assert_err!(terrain.remove("enroute/Europe/local.terrain"));
    assert!(!path.exists());
    assert_eq!(terrain.generation, 2);
    std::assert_matches!(terrain.files[path_id], TerrainSource::Unavailable(_));
    assert_ok!(fs::remove_dir(&marker));
    assert_ok!(terrain.remove("enroute/Europe/local.terrain"));
    assert!(terrain.files.is_empty());
    assert_eq!(terrain.generation, 3);
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
fn managed_identities_keep_same_name_terrain_independent() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let first = "enroute/Asia/Georgia.terrain";
    let second = "enroute/North America/United States/Georgia.terrain";
    for id in [first, second] {
        let path = root.join(id);
        assert_ok!(fs::create_dir_all(path.parent().unwrap()));
        write_terrain(&path, &[(7, 66, 87, &tagged_webp(id.as_bytes()))]);
    }
    let legacy = root.join("enroute/Georgia.terrain");
    write_terrain(&legacy, &[(7, 66, 87, &tagged_webp(b"legacy"))]);
    let mut terrain = assert_ok!(Terrain::load(root));
    let tile = terrain.resource_response("0/7/66/40.webp");
    assert_eq!(tile.body(), &tagged_webp(first.as_bytes()));
    assert_err!(terrain.set_enabled("Georgia.terrain", false));
    assert_ok!(terrain.set_enabled(first, false));
    let restarted = assert_ok!(Terrain::load(root));
    let tile = restarted.resource_response("0/7/66/40.webp");
    assert_eq!(tile.body(), &tagged_webp(second.as_bytes()));
    assert_ok!(terrain.remove(first));
    assert!(!root.join(first).exists());
    assert!(root.join(second).exists());
    assert!(legacy.exists());
}

fn terrain_entry(path: &'static str) -> CatalogEntry {
    CatalogEntry {
        path,
        country_code: "FR",
        continent: Continent::Europe,
        size: 1.try_into().unwrap(),
        publication_date: time::macros::date!(2026 - 09 - 08),
    }
}

fn terrain_download(directory: &Path, bytes: &[u8]) -> DownloadFile {
    use std::io::Write;
    let entry = terrain_entry("Europe/France.terrain");
    let mut download = assert_ok!(DownloadFile::new(directory, &entry));
    assert_ok!(download.file_mut().write_all(bytes));
    download
}

#[test]
#[tracing_test::traced_test]
fn downloaded_terrain_preserves_activation_and_retains_invalid_replacements() {
    let name = "enroute/Europe/France.terrain";
    for (installed, disabled) in [(false, false), (false, true), (true, false), (true, true)] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(name);
        assert_ok!(fs::create_dir_all(path.parent().unwrap()));
        if installed {
            write_terrain(&path, &[(7, 0, 0, &tagged_webp(b"old"))]);
        }
        if disabled {
            assert_ok!(fs::write(path.with_extension("terrain.disabled"), b""));
        }
        let mut terrain = assert_ok!(Terrain::load(directory.path()));
        let replacement = directory.path().join("replacement.terrain");
        let tile = tagged_webp(b"new");
        write_terrain(&replacement, &[(7, 0, 0, &tile)]);
        let download = terrain_download(directory.path(), &assert_ok!(fs::read(replacement)));
        assert_ok!(terrain.install_download(name, download));
        let disabled = installed && disabled;
        assert_eq!(terrain.generation, 1);
        assert_eq!(
            matches!(terrain.files[name], TerrainSource::Disabled),
            disabled
        );
        assert_eq!(path.with_extension("terrain.disabled").exists(), disabled);
        let response = terrain.resource_response("1/7/0/127.webp");
        let expected = if disabled { &[][..] } else { tile.as_slice() };
        assert_eq!(response.body(), expected);
        let download = terrain_download(directory.path(), b"invalid");
        assert_ok!(terrain.install_download(name, download));
        assert_eq!(assert_ok!(fs::read(&path)), b"invalid");
        assert_eq!(terrain.generation, 2);
        assert_eq!(
            matches!(terrain.files[name], TerrainSource::Unavailable(_)),
            !disabled
        );
        let restarted = assert_ok!(Terrain::load(directory.path()));
        assert_eq!(
            matches!(restarted.files[name], TerrainSource::Disabled),
            disabled
        );
    }
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
#[tracing_test::traced_test]
fn downloaded_terrain_rechecks_compatibility_and_refreshes_resources() {
    let directory = tempfile::tempdir().unwrap();
    let name = "enroute/Europe/France.terrain";
    let path = directory.path().join(name);
    assert_ok!(fs::create_dir_all(path.parent().unwrap()));
    write_terrain(&path, &[(7, 0, 0, &webp_header(256, 256))]);
    let germany = directory.path().join("enroute/Europe/Germany.terrain");
    write_terrain(&germany, &[(8, 0, 0, &webp_header(512, 512))]);
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    std::assert_matches!(
        terrain.files["enroute/Europe/Germany.terrain"],
        TerrainSource::Unavailable(_)
    );
    let replacement = directory.path().join("replacement.terrain");
    write_terrain(&replacement, &[(7, 0, 0, &webp_header(512, 512))]);
    let connection = Connection::open(&replacement).unwrap();
    assert_ok!(connection.execute("INSERT INTO metadata VALUES ('attribution', 'new')", []));
    drop(connection);
    let download = terrain_download(directory.path(), &assert_ok!(fs::read(replacement)));
    assert_ok!(terrain.install_download(name, download));
    std::assert_matches!(
        terrain.files["enroute/Europe/Germany.terrain"],
        TerrainSource::Active(_)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&assert_ok!(terrain.metadata())).unwrap();
    assert_eq!(metadata["tileSize"], 512);
    assert_eq!(metadata["maxzoom"], 8);
    assert_eq!(metadata["attribution"], "new");
    assert_eq!(
        terrain.resource_response("0/7/0/127.webp").status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        terrain.resource_response("1/8/0/255.webp").body(),
        &webp_header(512, 512)
    );
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
fn failed_terrain_installation_preserves_the_installed_file() {
    let directory = tempfile::tempdir().unwrap();
    let name = "enroute/Europe/France.terrain";
    let download = terrain_download(directory.path(), b"replacement");
    let path = directory.path().join(name);
    write_terrain(&path, &[(7, 0, 0, &tagged_webp(b"old"))]);
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    assert_err!(terrain.install_download("enroute/Europe/Germany.terrain", download));
    assert_eq!(terrain.generation, 0);
    let download = terrain_download(directory.path(), b"replacement");
    assert_ok!(remove_partial_downloads(directory.path()));
    assert_err!(terrain.install_download(name, download));
    assert_eq!(terrain.generation, 1);
    assert_eq!(
        terrain.resource_response("1/7/0/127.webp").body(),
        &tagged_webp(b"old")
    );
}

#[test]
#[tracing_test::traced_test]
fn terrain_installation_finishes_the_queue_and_preserves_failed_updates() {
    for failed in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let name = "enroute/Europe/France.terrain";
        let path = directory.path().join(name);
        let download = terrain_download(directory.path(), b"disabled replacement");
        assert_ok!(fs::write(&path, b"disabled original"));
        assert_ok!(fs::write(path.with_extension("terrain.disabled"), b""));
        let terrain = Mutex::new(assert_ok!(Terrain::load(directory.path())));
        if failed {
            assert_ok!(remove_partial_downloads(directory.path()));
        }
        let downloads = DownloadCommands::default();
        let attempt = {
            let mut queue = downloads.queue.lock().unwrap();
            queue.enqueue(terrain_entry("Europe/France.terrain"));
            assert_some!(queue.start_next())
        };
        let basemaps = Mutex::new(Basemaps::default());
        downloads.install_download(&basemaps, &terrain, &attempt, download);
        let status = downloads.queue.lock().unwrap().subscribe();
        assert_eq!(status.borrow().len(), usize::from(failed));
        if failed {
            std::assert_matches!(status.borrow()[0].state, DownloadState::Failed);
        }
        let expected = if failed {
            b"disabled original".as_slice()
        } else {
            b"disabled replacement"
        };
        assert_eq!(assert_ok!(fs::read(path)), expected);
        let terrain = terrain.lock().unwrap();
        assert_eq!(terrain.generation, 1);
        std::assert_matches!(terrain.files[name], TerrainSource::Disabled);
    }
    assert!(logs_contain("Could not install downloaded Enroute file"));
    assert!(!logs_contain("Could not open offline terrain"));
}

#[test]
fn available_updates_use_timestamps_for_active_and_disabled_terrain() {
    use std::fs::{FileTimes, OpenOptions};
    use std::time::Duration;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("enroute/Europe");
    assert_ok!(fs::create_dir_all(&root));
    let france = root.join("France.terrain");
    let germany = root.join("Germany.terrain");
    write_terrain(&france, &[]);
    assert_ok!(fs::write(&germany, b"disabled, not a database"));
    assert_ok!(fs::write(germany.with_extension("terrain.disabled"), b""));
    let entries = [
        "Europe/France.terrain",
        "Europe/Germany.terrain",
        "Europe/Malta.terrain",
    ]
    .map(terrain_entry);
    let publication: std::time::SystemTime = time::macros::datetime!(2026-09-08 0:00 UTC).into();
    let terrain = assert_ok!(Terrain::load(directory.path()));
    for offset in [-1i64, 0, 1] {
        let modified = if offset < 0 {
            publication - Duration::from_secs(1)
        } else {
            publication + Duration::from_secs(offset as u64)
        };
        for path in [&france, &germany] {
            let file = assert_ok!(OpenOptions::new().write(true).open(path));
            assert_ok!(file.set_times(FileTimes::new().set_modified(modified)));
        }
        let expected = if offset < 0 {
            vec!["Europe/France.terrain", "Europe/Germany.terrain"]
        } else {
            vec![]
        };
        assert_eq!(assert_ok!(terrain.available_updates(&entries)), expected);
    }
    assert_ok!(fs::remove_file(france));
    assert_err!(terrain.available_updates(&entries));
}

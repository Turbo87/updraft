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
    let france = tagged_webp(b"france");
    let germany = tagged_webp(b"germany");
    let west = tagged_webp(b"west");
    let east = tagged_webp(b"east");
    write_terrain(
        &directory.path().join("Germany.terrain"),
        &[(7, 66, 87, &germany), (10, 0, 0, &west)],
    );
    write_terrain(
        &directory.path().join("France.terrain"),
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
        (
            "attribution",
            "INSERT INTO metadata VALUES ('attribution', NULL)",
        ),
    ] {
        let path = directory.path().join(format!("{name}.terrain"));
        write_terrain(&path, &[]);
        Connection::open(path).unwrap().execute_batch(sql).unwrap();
    }
    let tile = webp_header(256, 256);
    write_terrain(
        &directory.path().join("valid.terrain"),
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
        let path = directory.path().join(format!("{name}.terrain"));
        std::assert_matches!(terrain.files[&path], TerrainSource::Unavailable(_));
        assert!(logs_contain(&format!("{name}.terrain")));
    }
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
fn missing_directory_is_empty_but_scan_failure_is_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("enroute");
    let terrain = assert_ok!(Terrain::load(&path));
    let response = terrain.resource_response("0/7/66/40.webp");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.body().is_empty());
    std::fs::write(&path, b"not a directory").unwrap();
    assert_err!(Terrain::load(&path).map(|_| ()));
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
    let path = directory.path().join("Germany.terrain");
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
    write_terrain(&directory.path().join("empty.terrain"), &[]);
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
        write_terrain(
            &directory.path().join("France.terrain"),
            &[(7, 0, 0, &webp_header(256, 256))],
        );
        write_terrain(
            &directory.path().join("Germany.terrain"),
            &[(zoom, 1, 0, &tile)],
        );
        let terrain = assert_ok!(Terrain::load(directory.path()));
        let rejected = directory.path().join("Germany.terrain");
        std::assert_matches!(terrain.files[&rejected], TerrainSource::Unavailable(_));
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
    let empty = directory.path().join("00-empty.terrain");
    write_terrain(&empty, &[]);
    let invalid = directory.path().join("01-invalid.terrain");
    assert_ok!(fs::write(&invalid, b"not sqlite"));
    for (name, size, zoom) in [("a", 256, 6), ("b", 512, 8), ("c", 256, 7)] {
        let path = directory.path().join(format!("{name}.terrain"));
        write_terrain(&path, &[(zoom, 0, 0, &webp_header(size, size))]);
        assert_ok!(
            Connection::open(path)
                .unwrap()
                .execute("INSERT INTO metadata VALUES ('attribution', ?1)", [name])
        );
    }
    let first = directory.path().join("a.terrain");
    let second = directory.path().join("b.terrain");
    let third = directory.path().join("c.terrain");
    assert_ok!(fs::write(first.with_extension("mbtiles.disabled"), b""));
    let terrain = assert_ok!(Terrain::load(directory.path()));
    assert_eq!(
        terrain.files.keys().collect::<Vec<_>>(),
        [&empty, &invalid, &first, &second, &third]
    );
    std::assert_matches!(terrain.files[&invalid], TerrainSource::Unavailable(_));
    std::assert_matches!(terrain.files[&first], TerrainSource::Active(_));
    std::assert_matches!(terrain.files[&second], TerrainSource::Unavailable(_));
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
        std::assert_matches!(terrain.files[&first], TerrainSource::Disabled);
        std::assert_matches!(terrain.files[&second], TerrainSource::Active(_));
        std::assert_matches!(terrain.files[&third], TerrainSource::Unavailable(_));
        assert_eq!(
            terrain.resource_response("0/8/0/255.webp").body(),
            &webp_header(512, 512)
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&assert_ok!(terrain.metadata())).unwrap();
        assert_eq!(metadata["attribution"], "b");
        assert_eq!(metadata["tileSize"], 512);
    }
    let added = directory.path().join("0-new.terrain");
    write_terrain(&added, &[(4, 0, 0, &webp_header(128, 128))]);
    let terrain = assert_ok!(Terrain::load(directory.path()));
    std::assert_matches!(terrain.files[&added], TerrainSource::Active(_));
    std::assert_matches!(terrain.files[&second], TerrainSource::Unavailable(_));
    for path in terrain.files.keys() {
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
    let path = directory.path().join("local.terrain");
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
    for (name, size) in [("active", 256), ("incompatible", 512)] {
        let path = directory.path().join(format!("{name}.terrain"));
        write_terrain(&path, &[(7, 0, 0, &webp_header(size, size))]);
    }
    let disabled = directory.path().join("disabled.terrain");
    assert_ok!(fs::write(&disabled, b"not sqlite"));
    assert_ok!(fs::write(disabled.with_extension("terrain.disabled"), b""));
    assert_ok!(fs::write(
        directory.path().join("invalid.terrain"),
        b"not sqlite"
    ));
    let terrain = Arc::new(Mutex::new(assert_ok!(Terrain::load(directory.path()))));
    let messages = Arc::new(Mutex::new(Vec::<Value>::new()));
    let received = messages.clone();
    let app = tauri::test::mock_builder()
        .manage(terrain.clone())
        .channel_interceptor(move |_, _, _, body| {
            let status = body.clone().deserialize().unwrap();
            received.lock().unwrap().push(status);
            true
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscribe_terrain,
            commands::unsubscribe_terrain,
            commands::set_terrain_enabled,
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
          "sourceName": "active.terrain",
          "type": "active"
        },
        {
          "sourceName": "disabled.terrain",
          "type": "disabled"
        },
        {
          "sourceName": "incompatible.terrain",
          "type": "unavailable"
        },
        {
          "sourceName": "invalid.terrain",
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
    let body = json!({"sourceName":"active.terrain", "enabled":false});
    assert_eq!(assert_ok!(invoke("set_terrain_enabled", body)), Value::Null);
    let delivered = messages.lock().unwrap();
    assert_eq!(delivered.len(), 3);
    insta::assert_json_snapshot!(delivered[2], @r#"
    {
      "generation": 1,
      "sources": [
        {
          "sourceName": "active.terrain",
          "type": "disabled"
        },
        {
          "sourceName": "disabled.terrain",
          "type": "disabled"
        },
        {
          "sourceName": "incompatible.terrain",
          "type": "active"
        },
        {
          "sourceName": "invalid.terrain",
          "type": "unavailable"
        }
      ]
    }
    "#);
    drop(delivered);
    let body = json!({"sourceName":"missing.terrain", "enabled":true});
    assert_eq!(
        invoke("set_terrain_enabled", body),
        Err(json!("Could not change terrain activation"))
    );
    assert_eq!(messages.lock().unwrap().len(), 3);
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
    let path = directory.path().join("local.terrain");
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
    let first = directory.path().join("a.terrain");
    let second = directory.path().join("b.terrain");
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
    assert_ok!(terrain.set_enabled("a.terrain", false));
    assert_eq!(terrain.generation, 1);
    assert!(first.with_extension("terrain.disabled").is_file());
    std::assert_matches!(terrain.files[&second], TerrainSource::Active(_));
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
    std::assert_matches!(restarted.files[&first], TerrainSource::Disabled);
    std::assert_matches!(restarted.files[&second], TerrainSource::Active(_));
    drop(restarted);
    assert_ok!(fs::write(&first, b"disabled files must not be opened"));
    assert_ok!(terrain.set_enabled("b.terrain", false));
    assert_eq!(
        terrain.resource_response("2/7/66/40.webp").status(),
        StatusCode::NOT_FOUND
    );
    let channel = Channel::new(|_| Err(std::io::Error::other("closed channel").into()));
    terrain.subscribers.insert(channel.id(), channel);
    assert_ok!(terrain.set_enabled("b.terrain", true));
    assert!(terrain.subscribers.is_empty());
    assert!(!logs_contain("a.terrain"));
    assert_ok!(terrain.set_enabled("a.terrain", true));
    std::assert_matches!(terrain.files[&first], TerrainSource::Unavailable(_));
    std::assert_matches!(terrain.files[&second], TerrainSource::Active(_));
    assert!(!first.with_extension("terrain.disabled").exists());
    assert_ok!(fs::remove_file(&first));
    write_terrain(&first, &[(7, 66, 87, &webp_header(256, 256))]);
    assert_ok!(terrain.set_enabled("a.terrain", true));
    std::assert_matches!(terrain.files[&first], TerrainSource::Active(_));
    std::assert_matches!(terrain.files[&second], TerrainSource::Unavailable(_));
    assert_eq!(
        terrain.resource_response("5/7/66/40.webp").body(),
        &webp_header(256, 256)
    );
    assert!(logs_contain("Could not open offline terrain"));
}

#[test]
fn activation_marker_failures_preserve_inventory_and_reject_unknown_names() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("local.terrain");
    write_terrain(&path, &[]);
    let mut terrain = assert_ok!(Terrain::load(directory.path()));
    let marker = path.with_extension("terrain.disabled");
    assert_ok!(fs::create_dir(&marker));
    for enabled in [false, true] {
        assert_err!(terrain.set_enabled("local.terrain", enabled));
        std::assert_matches!(terrain.files[&path], TerrainSource::Active(_));
        assert_eq!(terrain.generation, 0);
    }
    assert_ok!(fs::remove_dir(&marker));
    for name in [
        "missing.terrain",
        "../local.terrain",
        "nested/local.terrain",
    ] {
        assert_err!(terrain.set_enabled(name, false));
        assert_eq!(terrain.generation, 0);
    }
    assert_ok!(terrain.set_enabled("local.terrain", false));
    assert_ok!(terrain.set_enabled("local.terrain", false));
    assert_eq!(terrain.generation, 2);
}

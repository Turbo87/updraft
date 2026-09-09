use super::*;
use claims::{assert_ok, assert_some};
use image_webp::{ColorType, WebPEncoder};

#[test]
fn reuses_decoded_tiles() {
    let mut bytes = Vec::new();
    assert_ok!(WebPEncoder::new(&mut bytes).encode(&[128, 0, 0], 1, 1, ColorType::Rgb8));
    let connection = assert_ok!(Connection::open_in_memory());
    let schema = "CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);";
    assert_ok!(connection.execute_batch(schema));
    assert_ok!(connection.execute("INSERT INTO tiles VALUES (0, 0, 0, ?1)", [bytes]));
    let mut terrain = TerrainReader::default();
    let file = TerrainFile {
        connection,
        coverage: Some((1, 0, 0)),
        attributions: Vec::new(),
    };
    terrain.files.insert("a".into(), file);
    let first = assert_some!(assert_ok!(terrain.decoded_tile(0, 0, 0)));
    let second = assert_some!(assert_ok!(terrain.decoded_tile(0, 0, 0)));
    assert!(Arc::ptr_eq(&first, &second));
}

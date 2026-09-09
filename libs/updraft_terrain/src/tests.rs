use super::*;
use claims::{assert_ok, assert_some};

#[test]
fn reuses_decoded_tiles() {
    let mut bytes = Vec::new();
    assert_ok!(image_webp::WebPEncoder::new(&mut bytes).encode(
        &[128, 0, 0],
        1,
        1,
        image_webp::ColorType::Rgb8,
    ));
    let connection = assert_ok!(Connection::open_in_memory());
    assert_ok!(connection.execute_batch(
        "CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);"
    ));
    assert_ok!(connection.execute("INSERT INTO tiles VALUES (0, 0, 0, ?1)", [bytes]));
    let mut terrain = TerrainReader::default();
    terrain.files.insert(
        "a".into(),
        TerrainFile {
            connection,
            coverage: Some((1, 0, 0)),
            attributions: Vec::new(),
        },
    );
    let first = assert_some!(assert_ok!(terrain.decoded_tile(0, 0, 0)));
    let second = assert_some!(assert_ok!(terrain.decoded_tile(0, 0, 0)));
    assert!(Arc::ptr_eq(&first, &second));
}

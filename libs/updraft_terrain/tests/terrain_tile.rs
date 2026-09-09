use claims::{assert_err, assert_ok};
use image_webp::{ColorType, WebPEncoder};
use updraft_terrain::TerrainTile;

fn webp(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let encoder = WebPEncoder::new(&mut bytes);
    assert_ok!(encoder.encode(pixels, width, height, ColorType::Rgb8));
    bytes
}

#[test]
fn decodes_terrarium_without_rounding() {
    let bytes = webp(2, 2, &[127, 255, 128, 128, 0, 0, 128, 1, 0, 128, 2, 0]);
    let tile = assert_ok!(TerrainTile::decode(&bytes));
    assert_eq!(tile.elevation(0, 0), -0.5);
    assert_eq!(tile.elevation(1, 1), 2.0);
}

#[test]
fn rejects_corrupt_and_non_square_tiles() {
    assert_err!(TerrainTile::decode(b"invalid webp"));
    assert_err!(TerrainTile::decode(&webp(2, 1, &[128; 6])));
}

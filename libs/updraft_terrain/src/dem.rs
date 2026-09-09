use anyhow::{Context, Result, ensure};
use image_webp::WebPDecoder;
use std::io::Cursor;

#[derive(Debug)]
pub struct TerrainTile {
    pub pixels: Vec<u8>,
    pub size: u32,
    channels: usize,
}

impl TerrainTile {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut decoder = WebPDecoder::new(Cursor::new(bytes))?;
        let (width, height) = decoder.dimensions();
        ensure!(
            width == height && width > 0,
            "Terrain tiles must be nonempty squares"
        );
        let channels = if decoder.has_alpha() { 4 } else { 3 };
        let length = decoder
            .output_buffer_size()
            .context("Terrain tile dimensions overflow")?;
        let mut pixels = vec![0; length];
        decoder.read_image(&mut pixels)?;
        Ok(Self {
            pixels,
            size: width,
            channels,
        })
    }

    pub fn elevation(&self, x: u32, y: u32) -> f64 {
        let index = (y as usize * self.size as usize + x as usize) * self.channels;
        let rgb = &self.pixels[index..index + 3];
        f64::from(rgb[0]) * 256.0 + f64::from(rgb[1]) + f64::from(rgb[2]) / 256.0 - 32768.0
    }
}

pub fn bilinear(samples: [[f64; 2]; 2], x: f64, y: f64) -> f64 {
    let top = samples[0][0] * (1.0 - x) + samples[0][1] * x;
    let bottom = samples[1][0] * (1.0 - x) + samples[1][1] * x;
    top * (1.0 - y) + bottom * y
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use image_webp::{ColorType, WebPEncoder};

    pub fn webp(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let encoder = WebPEncoder::new(&mut bytes);
        assert_ok!(encoder.encode(pixels, width, height, ColorType::Rgb8));
        bytes
    }

    #[test]
    fn decodes_terrarium_and_interpolates_without_rounding() {
        let bytes = webp(2, 2, &[127, 255, 128, 128, 0, 0, 128, 1, 0, 128, 2, 0]);
        let tile = assert_ok!(TerrainTile::decode(&bytes));
        assert_eq!(tile.elevation(0, 0), -0.5);
        assert_eq!(tile.elevation(1, 1), 2.0);
        assert_eq!(bilinear([[-0.5, 0.0], [1.0, 2.0]], 0.5, 0.5), 0.625);
        assert_eq!(bilinear([[-0.5, 0.0], [1.0, 2.0]], 0.25, 0.75), 0.84375);
    }

    #[test]
    fn rejects_corrupt_and_non_square_tiles() {
        assert_err!(TerrainTile::decode(b"invalid webp"));
        assert_err!(TerrainTile::decode(&webp(2, 1, &[128; 6])));
    }
}

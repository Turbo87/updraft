use anyhow::{Context, Result, ensure};
use image_webp::WebPDecoder;
use std::io::Cursor;

/// A decoded square Terrarium tile with elevations relative to mean sea level.
#[derive(Debug)]
pub struct TerrainTile {
    /// Row-major RGB or RGBA bytes decoded from the WebP image.
    pub pixels: Vec<u8>,
    /// Width and height in pixels.
    pub size: u32,
    channels: usize,
}

impl TerrainTile {
    /// Decodes a nonempty square WebP image as a Terrarium elevation tile.
    ///
    /// Returns an error for malformed data, unsupported image features, or
    /// invalid dimensions. The input must use Terrarium encoding. WebP headers
    /// do not identify the elevation encoding.
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

    /// Returns the elevation in meters at a pixel, without interpolation.
    ///
    /// # Panics
    ///
    /// Panics if the pixel offset exceeds the decoded buffer. Both coordinates
    /// must be below `size`, and the buffer must retain its decoded layout.
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

    #[test]
    fn interpolates_without_rounding() {
        assert_eq!(bilinear([[-0.5, 0.0], [1.0, 2.0]], 0.5, 0.5), 0.625);
        assert_eq!(bilinear([[-0.5, 0.0], [1.0, 2.0]], 0.25, 0.75), 0.84375);
    }
}

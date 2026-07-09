//! Downsampling of the official 15′ `WW15MGH` source to the committed 1° grid.
//!
//! Compiled only under the `gen` feature, so the runtime library that
//! downstream crates depend on carries none of this. Regenerate the
//! committed grid with:
//!
//! ```text
//! cargo run -p updraft_egm96 --features gen
//! ```

use std::path::{Path, PathBuf};

/// Source rows in `WW15MGH.DAC` (90°N→90°S at 0.25°).
const SRC_ROWS: usize = 721;
/// Source columns in `WW15MGH.DAC` (0°E→359.75°E at 0.25°).
const SRC_COLS: usize = 1440;

/// Decimation factor from 0.25° to 1°.
const FACTOR: usize = 4;

/// Rows in the generated 1° grid.
const OUT_ROWS: usize = (SRC_ROWS - 1) / FACTOR + 1; // 181
/// Columns in the generated 1° grid.
const OUT_COLS: usize = SRC_COLS / FACTOR; // 360

/// Path to the committed official source grid.
pub fn source_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/WW15MGH.DAC")
}

/// Path to the committed downsampled grid embedded by the library.
pub fn committed_grid_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/egm96_1deg.bin")
}

/// Downsample the raw `WW15MGH.DAC` bytes to the 1° whole-metre grid the
/// library embeds.
///
/// Every fourth node is kept (integer-degree nodes coincide exactly with
/// 0.25° nodes, so this samples the true EGM96 undulation there rather than
/// averaging), converted from centimetres and rounded to a signed
/// whole-metre `i8`.
///
/// # Panics
///
/// Panics if `dac` is not the expected `SRC_ROWS × SRC_COLS × 2` bytes, or
/// if any sampled undulation rounds outside the `i8` range.
#[must_use]
pub fn downsample(dac: &[u8]) -> Vec<u8> {
    assert_eq!(
        dac.len(),
        SRC_ROWS * SRC_COLS * 2,
        "unexpected WW15MGH.DAC size"
    );

    let centimetres = |row: usize, col: usize| -> i16 {
        let i = (row * SRC_COLS + col) * 2;
        i16::from_be_bytes([dac[i], dac[i + 1]])
    };

    let mut grid = Vec::with_capacity(OUT_ROWS * OUT_COLS);
    for row in 0..OUT_ROWS {
        for col in 0..OUT_COLS {
            let cm = centimetres(row * FACTOR, col * FACTOR);
            let metres = (f64::from(cm) / 100.0).round() as i32;
            let byte = i8::try_from(metres).expect("undulation out of i8 range");
            grid.push(byte as u8);
        }
    }
    grid
}

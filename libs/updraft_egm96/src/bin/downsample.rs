//! Regenerates the committed 1° geoid grid (`data/egm96_1deg.bin`) from the
//! official 15′ `WW15MGH.DAC` source. Run after changing the source or the
//! downsampling logic:
//!
//! ```text
//! cargo run -p updraft_egm96 --features gen
//! ```

use updraft_egm96::downsample::{committed_grid_path, downsample, source_path};

fn main() -> std::io::Result<()> {
    let dac = std::fs::read(source_path())?;
    let downsampled = downsample(&dac);

    let out = committed_grid_path();
    std::fs::write(&out, downsampled)?;
    println!("Wrote {}", out.display());

    Ok(())
}

//! Regenerates the committed `updraft-sdf` spritesheet under
//! `frontend/static/sprites/` from the SVG sources. Run with
//! `cargo run -p updraft_sprites` after changing any sprite SVG.

fn main() -> anyhow::Result<()> {
    let output_dir = updraft_sprites::committed_output_dir();
    updraft_sprites::generate(&updraft_sprites::svg_source_dir(), &output_dir)?;
    println!("Wrote spritesheet to {}", output_dir.display());
    Ok(())
}

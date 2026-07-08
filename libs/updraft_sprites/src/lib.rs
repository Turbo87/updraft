//! Builds the `updraft-sdf` `MapLibre` spritesheet from the SVG sources in
//! `sprites/`. The generated `.png`/`.json` files are committed under
//! `frontend/static/sprites/` and served to `MapLibre` as an SDF sprite so
//! icons can be recolored and haloed at render time.

use anyhow::Context;
use spreet::{Sprite, Spritesheet, get_svg_input_paths, load_svg, sprite_name};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Base name of the generated spritesheet.
pub const SPRITE_NAME: &str = "updraft-sdf";

/// Directory holding the SVG sources for the spritesheet.
pub fn svg_source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("sprites")
}

/// Directory where the committed spritesheet is served from by the frontend.
pub fn committed_output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/static/sprites")
}

/// Generates the regular and retina SDF spritesheets from `svg_dir` into
/// `output_dir`.
pub fn generate(svg_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    generate_spritesheet(svg_dir, &output_dir.join(SPRITE_NAME), 1)
        .context("Failed to generate regular spritesheet")?;
    generate_spritesheet(svg_dir, &output_dir.join(format!("{SPRITE_NAME}@2x")), 2)
        .context("Failed to generate retina spritesheet")?;

    Ok(())
}

fn generate_spritesheet(svg_dir: &Path, output: &Path, pixel_ratio: u8) -> anyhow::Result<()> {
    let input_paths =
        get_svg_input_paths(svg_dir, false).context("Failed to read SVG input folder")?;

    let sprites = input_paths
        .iter()
        .map(|svg_path| {
            let tree = load_svg(svg_path).context("Failed to load SVG file")?;
            let sprite =
                Sprite::new_sdf(tree, pixel_ratio).context("Failed to create sprite from SVG")?;
            let name = sprite_name(svg_path, svg_dir)?;
            Ok((name, sprite))
        })
        .collect::<anyhow::Result<BTreeMap<String, Sprite>>>()?;

    let mut builder = Spritesheet::build();
    builder.sprites(sprites);
    builder.make_sdf();

    let spritesheet = builder
        .generate()
        .context("Failed to pack sprites into a spritesheet")?;

    spritesheet.save_spritesheet(output.with_extension("png"))?;
    let output = output
        .to_str()
        .context("Failed to convert output path to string")?;
    spritesheet.save_index(output, true)?;

    Ok(())
}

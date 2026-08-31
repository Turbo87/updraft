# Basemap glyphs

The map uses Barlow Semi Condensed for labels. Noto Sans supplies fallback
glyphs, including Greek and Cyrillic coverage. Each directory name lists the
complete MapLibre font stack in fallback order.

Each stack contains only ranges with glyph data. MapLibre renders an omitted
range locally and writes a warning. A loaded PBF remains authoritative, so
local fallback cannot supply glyphs that are missing from a retained range.

## Regeneration

Install `build_pbf_glyphs` 1.5.1. It requires FreeType.

```shell
cargo install build_pbf_glyphs --version 1.5.1 --locked
```

Download these Barlow Semi Condensed 1.408 TTF files from Google Fonts revision
`ade3d1533e06b2b1462ffcde8e08b129627ca360`. Use this base URL:

`https://raw.githubusercontent.com/google/fonts/ade3d1533e06b2b1462ffcde8e08b129627ca360/ofl/barlowsemicondensed/`

- `BarlowSemiCondensed-Medium.ttf`
- `BarlowSemiCondensed-MediumItalic.ttf`
- `BarlowSemiCondensed-Bold.ttf`

Download the Noto Sans 2.015 release archive from
`https://github.com/notofonts/latin-greek-cyrillic/releases/download/NotoSans-v2.015/NotoSans-v2.015.zip`.
Extract these files from `NotoSans/hinted/ttf`:

- `NotoSans-Medium.ttf`
- `NotoSans-MediumItalic.ttf`
- `NotoSans-Bold.ttf`

Put all six TTF files in one input directory. Run this command from the
repository root:

```shell
build_pbf_glyphs \
  --combinations frontend/static/basemap/fonts/combinations.json \
  <input-directory> \
  <output-directory>
```

Copy the three comma-separated stack directories from the output directory to
this directory. Do not copy the six single-font directories that the generator
also creates. Empty files from these stacks are at most 76 bytes. Remove them:

```shell
find frontend/static/basemap/fonts \
  -type f \
  -name '*.pbf' \
  -size -101c \
  -delete
```

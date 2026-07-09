# EGM96 source data

`WW15MGH.DAC` is the NGA **EGM96 15-arc-minute worldwide geoid-height
interpolation grid**:

- 721 rows (90°N → 90°S) × 1440 columns (0°E → 359.75°E) at 0.25° spacing
- big-endian `i16`, **centimetres** of geoid undulation above the WGS84
  ellipsoid (global range ≈ −107 m … +85 m)
- 2,076,480 bytes (721 × 1440 × 2)

It is a work of the U.S. government (National Geospatial-Intelligence
Agency) and is in the public domain. The file is committed so the grid can
be regenerated fully offline.

The runtime library does **not** embed this file — it embeds only the
small `egm96_1deg.bin` grid derived from it by the `gen` module.
Regenerate that derived grid with:

```sh
cargo run -p updraft_egm96 --features gen
```

Mirror used to obtain the file (original NGA distribution requires a
login): <https://download.osgeo.org/proj/vdatum/egm96_15/outdated/WW15MGH.DAC>

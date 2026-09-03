# Terrain

Status: Current behavior

Updraft shows hillshade and elevation colours automatically from offline
Enroute `.terrain` files.
The map uses the Igor method with MapLibre's default intensity and viewport
lighting. Shadows use black at 65% opacity. The hillshade sits before the
basemap's `waterway` layer. Terrain does not change the camera pitch or enable
a 3D surface.

Enroute files are a temporary source during development until Updraft can
generate and host its own terrain assets.

Elevation colours sit before the basemap's `water` layer, beneath land cover.
The colour ramp uses 50% opacity and interpolates linearly between elevation
stops. It uses white for lowlands, pale green and yellow for hills, tan and pink for
mountains, and pale grey and blue for high elevations. Both terrain layers
share one elevation source.

## Files and lookup

Place `.terrain` files in the application data directory's `enroute` folder.
Updraft opens them read-only at startup, in filename order. Restart the
application after changing files. Files must remain intact while it runs.

The reader requires MBTiles with WebP format metadata, Terrarium encoding,
and a compatible `tiles` table or view. It skips unsupported or invalid files
and logs a warning. A missing directory gives empty coverage. Other directory
scan failures also produce a warning.

Each request returns the first matching tile. Lookup converts XYZ coordinates
to TMS rows and does not filter by metadata bounds. The shell returns the
original WebP bytes. SQLite reads run on blocking workers outside the core.

## Rendering and metadata

The shell serves tiles at `updraft://localhost/terrain/{z}/{x}/{y}.webp`.
The `imagesize` parser reads the dimensions from each file's first WebP tile.
SQL reads the zoom limits from its `tiles` table. Files must use the same
square tile size. The source combines their zoom ranges. Empty files do not
contribute dimensions or zoom limits. MapLibre reuses the highest available level when
the camera zooms further in.

Missing tiles return HTTP 404 so MapLibre leaves those areas without terrain.
An empty image response would instead decode as an elevation sample. Read
failures return HTTP 500 and produce a warning.

The source reads TileJSON 3.0 metadata from
`updraft://localhost/terrain/metadata.json`. The document contains the tile
URL, zoom range, attribution, and MapLibre's `tileSize` and `encoding`
extension fields.
The frontend overrides the tile URL with Tauri's converted
URL for the current platform.

The endpoint combines the installed files' attribution entries. It removes
duplicates, empty entries, and Enroute's `None yet` placeholder. The About
screen shows the resulting credits. Tauri converts both resource URLs for
each platform.

This version does not provide numeric elevation queries, AGL calculations,
file import controls, downloads, or online fallback.

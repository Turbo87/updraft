# Terrain

Status: Current behavior

Updraft shows hillshade and elevation colours automatically from offline
Enroute `.terrain` files.
The map uses the Igor method with MapLibre's default intensity and viewport
lighting. Shadows use black at 65% opacity. The hillshade sits before the
basemap's `waterway` layer. Terrain does not change the camera pitch or enable
a 3D surface.

Elevation colours sit before the basemap's `water` layer, beneath land cover.
The colour ramp uses 50% opacity and interpolates linearly between elevation
stops. It uses white for lowlands, pale green and yellow for hills, tan and pink for
mountains, and pale grey and blue for high elevations. Both terrain layers
share one elevation source.

## Files and lookup

Terrain uses managed paths in application data, such as
`enroute/Europe/Germany.terrain`. The provider and complete catalog-relative path
identify a dataset. Updraft loads nested files at startup, in identity order.
Flat development files are ignored without migration. Symlinks are excluded.
Add data offers terrain alongside basemaps in the country catalog. Files are enabled
unless a sibling marker exists, such as `France.terrain.disabled` for
`France.terrain`. Disabled files are not opened or validated. Enabled files
are opened read-only. The library displays localized country and region names,
with a filename-stem fallback when catalog metadata is unavailable. Activation and
removal commands use the full identity, so equal filenames in separate regions
remain independent.

The reader requires MBTiles with WebP format metadata, Terrarium encoding,
and a compatible `tiles` table or view. The shell retains unsupported or invalid files
as unavailable and logs a warning. These files contribute no tiles or metadata.
A missing directory gives empty coverage. Other directory or marker scan
failures also produce a warning.

Each request returns the first matching tile. Lookup converts XYZ coordinates
to TMS rows and does not filter by metadata bounds. The shell returns the
original WebP bytes. SQLite reads run on blocking workers outside the core.

## Rendering and metadata

The shell serves tiles at
`updraft://localhost/terrain/{generation}/{z}/{x}/{y}.webp`.
At startup, the `imagesize` parser reads dimensions from each enabled file's
first WebP tile. SQL reads the zoom limits from its `tiles` table. Tiles must
be square. The first valid enabled file with tiles establishes the tile size.
Files with a different size remain enabled but unavailable. The source combines
the compatible files' zoom ranges and retains their validated metadata until
activation, installation, removal, or restart. Empty files do not establish a tile size or
contribute zoom limits.
MapLibre reuses the highest available level when the camera zooms further in.

Missing tiles return HTTP 404 so MapLibre leaves those areas without terrain.
An empty image response would instead decode as an elevation sample. Read
failures return HTTP 500 and produce a warning.

The source reads TileJSON 3.0 metadata from
`updraft://localhost/terrain/{generation}/metadata.json`. The document contains the tile
URL, zoom range, attribution, and MapLibre's `tileSize` and `encoding`
extension fields.
The frontend overrides the tile URL with Tauri's converted
URL for the current platform.
Both URLs identify the same inventory generation. Startup uses generation zero.
Requests for another generation return HTTP 404 without cached content. Invalid
generation values return HTTP 400. URLs without a generation are not supported.

The endpoint combines the active files' attribution entries. It removes
duplicates, empty entries, and Enroute's `None yet` placeholder. The About
screen shows the resulting credits. Tauri converts both resource URLs for
each platform.

The Data page lists terrain files with their enabled state and load errors.
The Enabled control saves the disabled marker and rechecks all enabled files in
filename order. An incompatible file can become active after another file is
disabled. Invalid files remain enabled and unavailable. Disabled files are not
opened. A marker write failure leaves the active inventory unchanged.

Each successful activation or installation publishes a new generation. The frontend
replaces the terrain source to discard cached tiles and cancel pending requests.
Tiles, metadata, and credits refresh together without changing the map camera.
Remove from device asks for confirmation. Removal closes the SQLite connection,
deletes the file and its disabled marker, then rechecks the remaining files.
A failure keeps the row for retry and publishes the current inventory. If the
file was deleted but marker cleanup failed, an enabled row becomes unavailable.
Retry also accepts an already missing file or marker.

## Downloads and updates

Country selection separates Basemap and Terrain. Each group starts unchecked.
Both types share one FIFO download queue. The library shows terrain progress,
Cancel, and Retry in the Terrain group. More files can be queued during a transfer.
A successful download returns an installed row without opening its details.

Terrain uses the same catalog cache and timestamp-based update detection as
[basemaps](basemap.md#downloads-and-updates). Settings and Data count updates
across both types. Installed rows show the actual size and modification date,
including for disabled files. Details label the date as Downloaded and retain
the Enroute source. Metadata read failures remain visible with Retry.

Downloads install atomically after transfer completion without database
validation. New files start enabled. Updates and Download again preserve the
installed activation state. Enabled files are loaded and checked for compatible
tile size. Invalid files replace the previous version and remain unavailable.
Disabled files stay unopened. Download again is available for unavailable files
that remain in the catalog, regardless of their timestamp.

Failed or cancelled transfers retain the installed file. Removal cancels a
pending replacement before deleting the file. Installation rechecks all enabled
terrain files and refreshes tiles, metadata, and credits together. It preserves
the map camera. Restart discards unfinished transfers and in-memory failures.

## Aircraft elevation

A shell worker samples terrain at the selected GPS position. Lookup does not
use map position, map zoom, or rendered tiles. It selects the highest active
zoom with coverage. At each zoom, filename order resolves overlapping files.

The reader decodes Terrarium WebP pixels and applies bilinear interpolation
between pixel centers. It uses adjacent tiles across tile edges. When an adjacent
tile is missing, it repeats the nearest available edge samples. Positions outside
the Web Mercator latitude range have no terrain elevation.

A Moka cache retains decoded tiles and missing-tile results. Its capacity is
weighted by decoded pixel bytes with a 16 MiB target. The limit is approximate.
Reads and decoding run on a blocking worker. Read failures produce a warning
and retry after five seconds. Position updates coalesce while a lookup or retry
is pending. Missing coverage does not trigger retries at an unchanged position.
An inventory change clears decoded and missing-tile cache entries and samples
terrain again, even when the aircraft position is unchanged.

The core accepts results only for the current selected position. It publishes
terrain elevation and calculates AGL from fused MSL altitude minus terrain
elevation. AGL remains unavailable when either input is unavailable. Negative
values remain visible. Terrain elevation follows position freshness. AGL is
stale when position or fused altitude is stale.

The debug overlay shows terrain elevation and AGL beside the fused altitude.
Both values use the selected altitude unit. Unavailable values show `–`.

This version does not provide custom file import controls or online fallback.

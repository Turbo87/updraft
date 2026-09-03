# Basemap

Status: Current behavior

Updraft uses offline vector basemaps from Enroute Flight Navigation. It does
not request online tiles. The application bundles its Positron style, fonts,
and sprites.

Enroute files are a temporary source during development until Updraft can
generate and host its own basemaps.

## Files and lookup

Place Enroute `.mbtiles` files in the `enroute` subdirectory of the application
data directory. Updraft scans this directory at startup and opens files
read-only, in filename order. Restart the application after changing files.
Files must remain intact while the application runs.

Each tile request returns the first file that contains the requested tile.
Updraft does not merge overlapping tiles. It converts XYZ row coordinates to
the TMS convention used by MBTiles and decompresses gzip PBF data in the shell.
File access and decompression run outside the core driver.

Lookup does not use geographic bounds. Tiles on either side of the antimeridian
use their global tile coordinates. Metadata bounds cannot exclude edge tiles.

The reader requires PBF format metadata and a compatible `tiles` table or view.
It skips files that cannot be opened or have an unsupported format or schema.
The log records these failures. A directory scan failure leaves the basemap
empty and produces a warning.

## Display

The style uses the Enroute zoom range of 6 through 10. The camera cannot zoom
out below level 6. MapLibre reuses zoom-10 tiles at higher camera zooms.

A missing directory, empty directory, or missing tile leaves the basemap
blank. Flight overlays remain available. Tile read and decompression failures
produce error responses and warnings instead of ordinary missing-tile responses.

The shell serves tiles under `updraft://localhost/basemap/{z}/{x}/{y}.pbf`.
The frontend converts the base URL through Tauri before it appends the tile
template. Android and Windows use the corresponding HTTP(S) URL with the
`updraft.localhost` host.

The About screen credits OpenStreetMap contributors, Enroute Flight Navigation,
and Akaflieg Freiburg. Zoom limits and attribution are fixed in the style.
The shell does not expose a basemap metadata endpoint.

## Excluded behavior

This version does not include file import controls, downloads, file watching,
online fallback, raster basemaps, or support for arbitrary vector tile schemas.

# Basemap

Status: Current behavior

Updraft uses offline vector basemaps from Enroute Flight Navigation. It does
not request online tiles. The application bundles its Positron style, fonts,
and sprites.

## Files and lookup

Basemaps use managed paths in application data, such as
`enroute/Europe/Germany.mbtiles`. The provider and complete catalog-relative path
identify a dataset. Updraft scans nested files at startup and opens enabled files
read-only. Flat development files in `enroute` are ignored without migration.
Symlinks are excluded. Files are installed through the managed download catalog.

Files are enabled by default. An empty `France.mbtiles.disabled` marker
beside `France.mbtiles` disables that basemap. Disabled files remain in the native inventory,
but Updraft does not open or validate them. Remove the marker to enable the file
on the next startup, or use the Enabled control in Data for an immediate change.
Terrain markers do not affect basemaps.

Each tile request returns the first enabled file in managed-identity order that contains
the requested tile.
Updraft does not merge overlapping tiles. It converts XYZ row coordinates to
the TMS convention used by MBTiles and decompresses gzip PBF data in the shell.
File access and decompression run outside the core driver.

Lookup does not use geographic bounds. Tiles on either side of the antimeridian
use their global tile coordinates. Metadata bounds cannot exclude edge tiles.

The reader requires PBF format metadata and a compatible `tiles` table or view.
Files that cannot be opened or have an unsupported format or schema remain in
the native inventory with their errors. They contribute no tiles. The log records
these failures. A directory scan or disabled-marker access failure leaves the
basemap empty and produces a warning.

## Library

Settings → Data lists installed basemap filenames and their disabled or
unavailable states. Select a file to see its enabled state and any load error.
Details also show Enroute as the source, installed file size, and modification
time as Downloaded. Metadata reads do not open disabled files. Read failures
offer Retry. The current catalog date does not identify the installed version.
Commands use the full managed identity. Equal filenames in different regions
have independent activation and removal state.
The Enabled control saves the choice and refreshes the map. Disabled files are
not opened or validated. Enabling an invalid file keeps it enabled and shows
its load error. Marker-write failures keep the confirmed activation state.
Remove from device asks for confirmation, then deletes the file and its disabled
marker. The map falls back to remaining files. A removal failure allows retry.
If marker cleanup fails after deletion, the file stays listed until cleanup
succeeds. Its deleted tiles no longer contribute to the map.

The app maintains the native status subscription across navigation. A subscription
failure shows an error instead of an empty library.

## Downloads and updates

Add data opens the Enroute country catalog. All dataset selections start
unchecked. Current installed files cannot be selected again. Available updates
can be selected, including updates to disabled files. Download accepts the
selection into one FIFO queue and returns to the library. More files can be
queued while a transfer runs.

The app refreshes the cached catalog at startup. A failed check retains the
cache and last successful check time and offers Retry. Nothing polls while the
library is open. A catalog publication date after the installed modification
time means an update is available. Size differences do not indicate updates.
Updates are manual. The library notice opens Updates, where each row offers
Update and Update all skips queued or active transfers. Settings shows the
available count. No available updates produce no notice or count.

Downloads stream to temporary files and install atomically. Byte-progress
notifications are limited to one per 100 ms. Queue changes publish immediately.
Cancel discards the partial file and advances the queue. Transfer or write
failure preserves the installed file. Failed rows offer Retry and remain until
success or app restart. Restart does not restore the queue or resume transfers.

Installation does not validate the database first. New files start enabled.
Updates and Download again retain activation. Enabled invalid files remain
installed with a load error. Disabled files stay unopened. Unavailable basemap
details offer Download again when the file remains in the catalog, without an
update-date requirement. Removal cancels a queued or active replacement before
deleting the installed file. Installation refreshes map resources and preserves
the camera.

Download-only Android background execution without a working location or
connected-device session is unsupported. Background and screen-lock validation
on a physical Android device remains pending.

## Display

The style uses the Enroute zoom range of 6 through 10. The camera cannot zoom
out below level 6. MapLibre reuses zoom-10 tiles at higher camera zooms.

A missing directory, empty directory, or missing tile leaves the basemap
blank. Flight overlays remain available. Tile read and decompression failures
produce error responses and warnings instead of ordinary missing-tile responses.

The shell serves tiles under `updraft://localhost/basemap/{generation}/{z}/{x}/{y}.pbf`.
The frontend converts the base URL through Tauri before it appends the tile
template. Android and Windows use the corresponding HTTP(S) URL with the
`updraft.localhost` host.

Activation and removal advance the generation. Requests for other generations return no
content. The frontend replaces the basemap source to discard cached tiles and
cancel pending requests. It restores the layer order and preserves the camera.

The About screen credits OpenStreetMap contributors, Enroute Flight Navigation,
and Akaflieg Freiburg. Zoom limits and attribution are fixed in the style.
The shell exposes installed file size and modification time through a command.

## Excluded behavior

This version does not include custom basemap import, file watching,
online fallback, raster basemaps, or support for arbitrary vector tile schemas.

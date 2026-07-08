# Offline Data & Storage

The system is offline-first: vector map tiles, terrain, airspace, and waypoints all work with zero connectivity.

## Basemap (Vector Tiles)

Offline-first via PMTiles/MBTiles packs stored on device, served to MapLibre through the bulk geodata path (see [core.md](core.md)).

## Terrain

**Single source for rendering and computation.** Leading candidate: terrain-RGB raster tiles built from open DEM data. MapLibre consumes them natively for hillshade/3D, and the terrain library decodes the same tiles Rust-side with an LRU tile cache for AGL and glide-path sampling.

## Aviation Data

The user **imports** files via OS file picker or share intent. OpenAir (airspace) and CUP (waypoints/airfields, tasks) files are supported as a baseline. Other file formats will be implemented as needed.

In a later phase we will support in-app downloads from e.g. openAIP with update notifications, where possible.

## Storage Layout

```
<app-data>/
├── basemaps/
├── terrain/
├── airspace/
├── waypoints/
├── tasks/
├── flights/     # IGC logs
├── state/       # in-flight resume snapshots (active task, logging status)
├── captures/    # raw NMEA captures + replay input recordings (opt-in)
└── config/      # settings, device profiles, UI state
```

Note that CUP files contain waypoints and (optionally) tasks too. These files should be supported in `waypoints/` and `tasks/` directories.

Glider polars ship built-in as embedded data, with user overrides in `config/`.

## Crash-Safe Persistence

A mid-flight crash or OS process kill must not lose the flight:

- IGC logs are written incrementally and flushed per fix batch (append + flush).
- The core snapshots in-flight state periodically. Snapshot writes are atomic (write-temp + rename).
- On startup the app detects an interrupted flight and resumes logging automatically.

## Open Questions

- **Exact terrain tile format:** decision criteria: one file per region, decodable in Rust without GDAL, ~30–90 m resolution, MapLibre-compatible.
- **PMTiles vs MBTiles** for packs: depends on the tiling toolchain and the Rust crate ecosystem.

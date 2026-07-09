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
├── aircraft/
├── flights/     # IGC logs
├── state/       # in-flight resume snapshots (active task, logging status)
├── captures/    # raw NMEA captures + replay input recordings (opt-in)
└── config/      # settings, device profiles, client UI preferences
```

Note that CUP files contain waypoints and (optionally) tasks too. These files should be supported in `waypoints/` and `tasks/` directories.

Aircraft presets (glide polars plus mass, wingspan, and related data) ship built-in as embedded data; user aircraft profiles live in `aircraft/`.

## Crash-Safe Persistence

A mid-flight crash or OS process kill must not lose the flight:

- The flight module produces ordered IGC bytes and tracks their sequence. A native `FlightLogWriter` appends them without blocking the application loop.
- Resumable flight state is a versioned snapshot with a checksum, active dataset identities, and the last durable flight-log sequence.
- Snapshot writes use atomic replacement. The required durability points are explicit rather than implied by every ordinary flush.
- On startup the runtime loads the latest valid snapshot, reconciles it with the existing IGC file, and resumes logging before accepting live inputs.

## Open Questions

- **Exact terrain tile format:** decision criteria: one file per region, decodable in Rust without GDAL, ~30–90 m resolution, MapLibre-compatible.
- **PMTiles vs MBTiles** for packs: depends on the tiling toolchain and the Rust crate ecosystem.
- **Durability policy:** which transitions require an `fsync`-equivalent operation on Android, iOS, and desktop filesystems.

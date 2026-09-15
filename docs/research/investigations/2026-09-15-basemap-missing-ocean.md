# Cut-off water areas in Enroute basemaps

Status: Historical investigation

This investigation examined sea areas that end at straight lines in the
offline basemap, for example the Gulf of Lion, the coast of Sardinia, and the
Gulf of Taranto. It used the published Enroute basemaps dated 2026-09-14 for
Italy, France, and Germany, the `enrouteServer` repository at commit
`5783f95`, and tilemaker 3.2.0 built from source on Linux.

## Conclusion

The published basemaps contain no ocean polygons. The visible water shapes with
straight edges are OpenStreetMap `natural=bay` polygons. A bay polygon follows
the coastline and closes with one straight segment across the mouth of the bay.

The Enroute pipeline reads the ocean from the OpenStreetMap water polygons
shapefile. Tilemaker skips a shapefile layer without an error when it cannot
open the file. It prints a shapelib message and exits with status 0. The
published maps are consistent with a build in which no shapefile source was
available. The maps also lack the other three shapefile layers: Natural Earth
urban areas, glaciers, and ice shelves.

Updraft does not cause the defect. The client renders the tiles it receives.

## Evidence

1. In `Italy.mbtiles`, `France.mbtiles`, and `Germany.mbtiles`, the `water`
   layer contains `ocean` features only along coastlines. Open-sea tiles at
   zoom 10 contain no full-tile ocean polygon. The `landuse` layer, which the
   pipeline fills only from the urban-areas shapefile, is empty in all three
   files.
2. The pipeline for Sardinia was run locally from a fresh OpenStreetMap extract.
   With the shapefiles present, tilemaker reported 195 external polygons and
   produced a complete sea. With the `data` directory absent, tilemaker
   reported 0 external polygons, exited with status 0, and produced a map with
   bays only.
3. The `water` features of four published Italy tiles over Sardinia (zoom 7,
   8, and 10) were identical in class, bounds, and area to the local build
   without shapefiles, and different from the local build with shapefiles.
4. In `process.lua`, `natural=bay` polygons receive class `ocean`. The pipeline
   drops a bay only when the ocean shapefile covers at least 98% of it. Without
   the shapefile, every bay survives. With the shapefile present, a build
   without the bay rule produced the same ocean feature counts at every zoom
   level, so the rule adds nothing visible when the ocean data is present.

## Eliminated causes

- Tile lookup in Updraft: the first-file-wins rule cannot remove features
  from a tile that no file contains.
- Tilemaker simplification, polygon union, and tile clipping: the straight
  edges are input geometry from the bay polygons, and the coastline vertices
  in the published tiles are not simplified.
- The 20 km country buffer in `removeForeignTiles`: this deletes whole tiles
  far from the coast. It produces axis-aligned gaps at zoom 9 and 10 only. It
  is a secondary effect that also affects a correct build.

## Candidate fixes

All candidates were tested with the local Sardinia build. The tested patches
for fixes 1 to 3 are in
[2026-09-15-basemap-missing-ocean](2026-09-15-basemap-missing-ocean/). The
`enrouteserver-*` patches apply to `enrouteServer` commit `5783f95`. The
`tilemaker-*` patch applies to tilemaker 3.2.0.

1. `enrouteServer`: check that every tilemaker layer `source` exists before
   tilemaker runs, and stop with an error that names the missing files. The
   check passed with the data present and raised with the data absent.
2. tilemaker: throw when `SHPOpen` or `DBFOpen` fails instead of returning.
   The patched build exited with status 255 and wrote no output without the
   data. With the data, it produced the same per-layer feature counts as the
   unpatched build.
3. `enrouteServer`: keep tiles inside the region bounding box that contain
   only ocean polygons, instead of deleting them with the country buffer. This
   closes the axis-aligned gaps far from the coast. In the Sardinia test the
   retained tiles averaged about 130 bytes each.
4. `process.lua`: remove the `natural=bay` fallback. This makes a missing
   ocean obvious instead of drawing partial bays. It does not change a
   correct build. It is optional.

Fix 1 or 2 addresses the root cause. Fix 3 is independent and improves a
correct build. Updraft needs no change.

## Limits

- Older published maps were not available for comparison. The report that
  older maps showed the same defect was not verified.
- The build environment of the Enroute server was not inspected. The evidence
  identifies missing shapefile sources during the build, not why they were
  missing.
- The Sardinia test used a regional OpenStreetMap extract from
  `download.openstreetmap.fr`, not the continent file that the pipeline uses.

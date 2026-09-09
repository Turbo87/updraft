# Terrain elevation benchmarks

Run the suite with the release profile:

```sh
cargo bench -p updraft_terrain --bench elevation
```

The benchmarks use the public `TerrainReader` API and the checked-in
[France fixtures](../../../testdata/README.md). CSV parsing occurs
outside the timed work. Missing elevations and read failures stop the benchmark.

## Workloads

- `warm/interior` samples four positions within one decoded tile. Reported time
  is for all four lookups. Throughput counts individual lookups.
- `decoded_cache_miss/interior` samples one position with a fresh reader for each
  iteration. Criterion excludes reader creation, file opening, validation, and
  reader destruction from the measured lookup. The measurement includes SQLite
  tile lookup, WebP decoding, cache insertion, and interpolation. The operating
  system can cache file pages, so this is not a cold-disk benchmark.
- `warm/edge` and `warm/corner` sample boundaries with neighbouring tiles present.
  `warm/coverage_edge` and `warm/coverage_corner` sample a quarter pixel inside
  the patch boundary, where missing neighbours require edge fallback. Each case
  warms its required tiles and missing-tile entries before measurement.
- `warm/flight_segment` samples all 225 recorded positions in order. One untimed
  pass warms the cache. Each iteration replays the complete segment without
  waiting between fixes. Reported time is for the full segment. Throughput counts
  individual lookups.

The cache-miss benchmark uses `iter_batched_ref()` with `BatchSize::PerIteration`
to keep only one fresh reader and SQLite connection alive at a time. Criterion's
per-iteration timing overhead is included. The suite does not measure cache
pressure, eviction, source activation, or file discovery.

## Comparisons

Save a baseline before changing the implementation:

```sh
cargo bench -p updraft_terrain --bench elevation -- --save-baseline before
```

Compare the new implementation on the same machine with the same build and
measurement settings:

```sh
cargo bench -p updraft_terrain --bench elevation -- --baseline before
```

Criterion stores local results under `target/criterion`. Keep the fixture and
position sequence unchanged when comparing implementations. Repeat measurements
before treating a difference as a regression. The suite has no CI timing gate.
For a quick correctness check without collecting timings, use:

```sh
cargo bench -p updraft_terrain --bench elevation -- --test
```

# Arrival Height Verification

Status: Partial verification record

## Backend measurement

An optimized Rust build on an arm64 macOS host measured
`ArrivalResource::calculate()`. This includes viewport selection, glide solving,
feature construction, and GeoJSON serialization. It excludes catalog parsing,
core queries, worker scheduling, URL delivery, and map rendering.

The synthetic input used the default LS8 polar, zero bugs and ballast, a 200 m
reserve, MC 1 m/s, and wind from 270 degrees at 10 m/s. A GNSS fix at 50 degrees
north, 6 degrees east supplied an ellipsoid altitude of 2,000 m. Sensor fusion
provided the MSL altitude used by the solver.

Each catalog contained only grass airfields at 100 m MSL. Names were `Field N`.
For zero-based index `N`, latitude minutes were `(N % 100) * 0.5` above
50 degrees north. Longitude minutes were `((N / 100) % 100) * 0.5` above
6 degrees east, with integer division. The viewport covered latitudes 49 to 51
and longitudes 5 to 7, so every field was selected. The output feature count
was checked before timing.

Each size had five warm-up calls and 21 measured calls. The median and
nearest-rank 95th percentile were:

- 100 landables: 0.711 ms median, 0.857 ms p95, 26,426 output bytes.
- 1,000 landables: 4.445 ms median, 5.140 ms p95, 262,229 output bytes.
- 10,000 landables: 47.193 ms median, 48.438 ms p95, 2,638,985 output bytes.

These samples fit within the 100 ms viewport calculation interval on this host.
They do not establish end-to-end update latency or Android performance. The
10,000-landable response also requires substantial parsing and rendering work
that this measurement excludes. Other applications, including a native replay,
were running during the measurement. The temporary measurement harness was
removed after the run.

## Native and Android checks

The user confirmed that arrival heights appear during native desktop replay.
Browser regression tests cover changing labels with fading enabled, fixed-anchor
availability, and the arrival offset. Native UI tooling could not address the
running application, so an independent visual check was not completed.

No Android device was available. Physical Android verification is planned with
the nightly build after merge. Final native layout acceptance, dense-label
readability, sustained update latency, and Android behavior remain unverified.

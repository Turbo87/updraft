# Parser fixtures

Some of these files are unmodified real-world inputs used to exercise Updraft's parsers. Keep their original bytes intact. Line endings, checksums, signatures, extensions, and unusual records are part of the fixture coverage.

- `euregiocup_2026.cup`: CUP waypoints from the [EuregioCup 2026 downloads](https://www.soaringspot.com/de/euregiocup-2026/downloads).
- `airspace/euregiocup_2026.txt`: OpenAir airspace from the [EuregioCup 2026 downloads](https://www.soaringspot.com/de/euregiocup-2026/downloads).
- `flight_1.nmea`: Real-life FLARM traffic scenario with multiple targets, copied byte-for-byte from an [XCSoar fixture](https://github.com/XCSoar/XCSoar/blob/b9ab9ca951552c759b7863fe09e01c2ac94bfea3/test/data/driver/FLARM/rl-traffic.nmea) introduced in [XCSoar commit `b9ab9ca`](https://github.com/XCSoar/XCSoar/commit/b9ab9ca951552c759b7863fe09e01c2ac94bfea3). XCSoar distributes its repository under `GPL-2.0-or-later`. This copied fixture is not covered by Updraft's `MIT OR Apache-2.0` license.
- `weglide_1141558.igc`: IGC flight downloaded from [WeGlide flight 1141558](https://www.weglide.org/flight/1141558) and supplied by the pilot. It includes engine usage near the end of the flight.

## OpenAIP fixtures

The files in `openaip/` are selections, not complete exports. Each file keeps
whole records copied from the daily OpenAIP JSON exports of 2026-08-10 at
`https://s3.openaip.net/openaip-system-exports/<country>_<dataset>.json`. The
records are re-indented for review. Each file holds the smallest set of records
that still covers every field that the source datasets use.

- `openaip/airspaces.json`: three records from `de_asp.json`.
- `openaip/airspace_multi_ring.json`: one record from `us_asp.json`. Its polygon
  has two rings. The published schema permits one ring.

# Derived airspeed and wind have no age limit

Without an airspeed source, Updraft derives airspeed from ground velocity and
the wind. Derived airspeed and the reported wind stay available while the wind
filter is converged after its last accepted measurement. The wind age has no
limit. A new estimate that is not converged yet, or a reset of the fusion,
ends them. With an airspeed source, the behavior does not change.

The decision rests on the replay bench, which removed TAS from recorded flights
with TAS. In cruise, Vario and Netto from an old wind had a lower RMS against
the Reference than Vertical speed and Estimated netto in every wind-age bucket
up to 90 min. The old wind also had a smaller vector error than calm air,
which the arrival calculation uses when the wind is missing.

| Wind age | Vario | Vertical speed | Netto | Estimated netto | Wind | Calm air |
|---|---|---|---|---|---|---|
| 0–2 min | 0.82 | 1.63 | 0.87 | 1.63 | 3.6 | 9.7 |
| 5–10 min | 0.74 | 1.58 | 0.79 | 1.60 | 4.6 | 10.1 |
| 20–40 min | 0.68 | 1.32 | 0.76 | 1.45 | 9.0 | 17.3 |
| 40–60 min | 0.66 | 1.29 | 0.77 | 1.37 | 12.0 | 23.7 |
| 60–90 min | 0.84 | 1.58 | 0.88 | 1.63 | 6.2 | 31.7 |

All values are cruise RMS in m/s. The bench has no data after 90 min, and the
60–90 min bucket comes from only one or two flights.

## Considered options

- The previous variance gate dropped the wind at a median of 4.6 min after the
  last measurement. 18.3% of the cruise after the first wind had no derived
  values. The variance also did not match the real error: a fresh wind had a
  vector error of 2.8 m/s RMS, but the converged variance implied about 1 m/s.
- An age limit of 20 or 40 min removes values that beat the fallbacks.

| Gate | Available | Vario shown | Netto shown |
|---|---|---|---|
| Variance gate | 79.9% | 0.83 | 0.90 |
| Converged, age ≤ 20 min | 80.4% | 0.87 | 0.93 |
| Converged, age ≤ 40 min | 90.2% | 0.78 | 0.84 |
| Converged, no age limit | 92.6% | 0.76 | 0.82 |
| No gate | 99.2% | 0.75 | 0.80 |

- No gate gives slightly better numbers, but a wind that did not converge
  after its last measurement loses to the fallbacks. On those samples, Vario
  had 1.15 m/s RMS and Vertical speed 1.03 m/s.

## Consequences

- All consumers of the reported wind follow the gate: the wind infoboxes, the
  arrival margins, the traffic climb compensation, the hillshade lighting, and
  the flight recording.
- A restored wind counts as a measurement at the restart time. It stays
  reported until a new estimate replaces it.
- The measured numbers keep the circle-fit policy, which allows partial arcs
  and resets the filter at a full circle while the variance is not converged.
  A change of that policy needs a new bench run.

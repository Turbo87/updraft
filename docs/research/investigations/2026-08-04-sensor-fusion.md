# Vario and wind from recorded flights

This investigation checks how well Updraft can estimate total-energy vertical
speed, wind, and netto. The available inputs are GNSS, pressure altitude, and,
when connected, true airspeed.

The main results are:

- Total-energy vertical speed stays close to the recorded instrument values.
- Wind is most reliable while the aircraft turns or supplies true airspeed.
- A quarter turn can recover an initial wind estimate during wave flight.
- Recorded netto is not a ground truth. On several aircraft, it changes with
  the direction of turn and can imply a negative sink rate.
- A fixed two-second vario smoothing time remains the safe choice.

The results support the sensor-fusion code in `updraft_core`. They do not prove
that the recorded instrument values are correct.

## Data and method

The current corpus has 16 LXNAV IGC recordings. It covers seven glider types.
Four recordings are wave flights. The others are cross-country or local soaring
flights. All recordings come from one instrument manufacturer.

Two recordings are repository fixtures:
`testdata/weglide_1141558.igc` is a cross-country flight, and
`testdata/weglide_1015312.igc` is a wave flight. The other source recordings
are retained privately because they can contain personal data. Public WeGlide
flight numbers identify them in the tables below. The repository owner can
provide source recordings when their use is appropriate.

The replay sends every sample to the estimator. Scoring starts after take-off.
It excludes engine use and the next minute. The recorded total-energy vario and
wind are references. The instruments use a total-energy probe and an inertial
platform that Updraft does not have.

The tables use root-mean-square (RMS) difference. RMS gives more weight to a
few large differences than to many small ones. The per-flight sample count is
therefore also important, especially for wave flight.

## Main comparison

With recorded true airspeed, the cross-country flights have a total-energy
vertical-speed RMS difference of 0.25 to 0.92 m/s. Wind-speed RMS is 1.13 to
2.47 m/s. Wind-direction RMS is 13.53° to 72.34°.

The four wave flights have a total-energy vertical-speed RMS difference of
0.25 to 0.46 m/s. Wind-speed RMS is 2.27 to 6.21 m/s. Wind-direction RMS is
11.70° to 36.74°. Wind changes much more with altitude in wave. A time filter
cannot follow that change without a new measurement.

Large direction differences usually occur in weak wind. A small change in the
wind vector then produces a large change in direction. The speed and direction
figures must be read together.

Without true airspeed, total-energy vertical-speed RMS rises to 0.59 to
1.32 m/s. Wind-speed RMS rises to 1.42 to 9.83 m/s. The estimator can still
recover useful values from the flight path, but it needs turns and reports less
often.

## Total-energy vertical speed

An aircraft can trade height for speed without moving up or down in the air.
Pressure altitude alone reads that trade as a climb or descent. The estimator
adds the kinetic-energy term to altitude before it calculates vertical speed:

```text
total-energy altitude = altitude + airspeed² / (2 × gravity)
```

The derivative of this value is total-energy vertical speed. Two filters, each
with a two-second time constant, smooth the result.

Pressure altitude supplies fast changes. GNSS altitude supplies slow changes.
This combination improves the vertical-speed result by about 12% on the five
recordings used for the altitude comparison. It also avoids dependence on an
altimeter setting. The
[altitude-reference investigation](2026-08-31-altitude-references.md) records
the source comparison and the rejected atmospheric inputs.

The one-second recordings cannot reproduce all movement of a faster vario.
This explains much of the short-term RMS difference. In an earlier set of 350
climbs lasting at least two minutes, the average climb-rate difference was
0.05 m/s. Its RMS was 0.07 m/s, and the largest difference was 0.29 m/s. The
short-term disagreement largely averages away over a complete climb.

## Wind with measured airspeed

Each sample supplies ground speed, track, and true airspeed. The estimator
finds the wind vector that explains their difference. One straight sample only
constrains wind along the current heading. A change of heading adds information
from another direction.

The wind filter keeps one continuous estimate. Airspeed samples update it in
straight flight and in turns. Circle measurements use the same filter when an
airspeed sample is absent. This lets an airspeed source connect or disconnect
without replacing the wind state.

The GNSS accuracy value can increase the assumed measurement error. It cannot
make the measurement more trusted than the normal baseline. The filter reports
wind only after its uncertainty falls below the reporting limit.

Wind is less accurate in straight flight without an airspeed measurement. The
crosswind part is then not observable. The filter can only carry the last
measured value forward until the aircraft turns again.

## Wind without an airspeed sensor

During a steady circle, the ground-velocity points form a circle. Its centre is
the wind vector. Its radius is the airspeed. Fitting both values is weaker than
using measured airspeed, but it gives phones and simple FLARM devices a useful
fallback.

A complete circle remains the normal measurement. It has strong geometry and
can update an existing wind estimate. A partial arc means only part of a turn.
It gives a weaker measurement because many circle centres can fit a short arc.

Wave flights showed why a partial arc is still useful. They contain long
straight runs and few complete circles. The estimator now accepts a 90° arc
only while no wind is reportable. It gives that measurement six times the
variance of a full-circle measurement. Once wind is available, partial arcs no
longer change it.

The first 12 flights selected the 90° arc and its variance. Three later wave
flights were holdouts. They tested the selected values without another sweep.

| Holdout result | Full circles | Partial-arc recovery |
| --- | ---: | ---: |
| Wind reported | 19.82% | 34.37% |
| Speed RMS | 7.70 m/s | 6.55 m/s |
| Direction RMS | 34.44° | 31.94° |

The partial-arc rule kept all 3,741 baseline samples. It added 2,747 samples.
The added samples had a speed RMS of 4.67 m/s and a direction RMS of 28.77°.
The extra values were useful measurements, not only extra noise.

## Netto and the recorded reference

Netto is total-energy vertical speed plus the aircraft's expected sink rate.
The estimator reads that sink rate from the glider polar. It adjusts the polar
for air density and for the extra load in a turn.

The recorded netto is not a direct measurement of rising air. Subtracting the
recorded vario from the recorded netto shows the sink rate that the instrument
applied. That value should not depend on whether the aircraft turns left or
right.

The recordings do not meet this condition. Flights 1141558 and 1179475 imply a
negative sink rate in left turns. Flight 1179605 implies one in right turns.
Several other flights show large left-to-right differences. The sign stays
consistent across repeated flights by the same aircraft. This points to an
aircraft installation effect, such as pressure-port position or steady
sideslip, rather than a shared software error.

When the wings are level, the recorded sink rate is 0.67 to 1.23 m/s in the
current corpus. This agrees with the range expected from the glider polars.
The recorded netto remains useful as supporting evidence, but its turn bias
prevents it from serving as ground truth.

## Rejected experiments

### Adaptive vario smoothing

An experiment estimated altitude noise and changed the two-second smoothing
time. It connected two calibration points: about 0.6 m of noise with a
two-second time constant, and 0.024 m with a 0.25-second time constant.

The experiment was not kept. Noise alone cannot show whether a sensor driver
has already removed useful movement and added delay. The one-second IGC fixture
also used the fixed fallback, so it did not test the adaptive path. The fixed
two-second value remains the accepted behavior. The
[device-barometer investigation](2026-08-29-device-barometer-response.md)
records the bench measurements and the evidence needed to revisit the choice.

### Delayed GNSS altitude detection

One receiver logged GNSS altitude about one second late. Combining that delayed
value with current pressure altitude made its vertical-speed result 32% worse.
An experiment tried to detect this delay from recent altitude rates.

The detector was not kept. It added state and special cases for one observed
source. Timestamp checks already reject altitudes that arrive too far apart.
Device adapters must assign correct timestamps before sensor fusion. The
[altitude-reference investigation](2026-08-31-altitude-references.md#delayed-gnss-altitude)
records the measured delay and the adapter boundary.

### Wind uncertainty from altitude change

An experiment increased wind uncertainty when the aircraft changed altitude.
This could let the filter follow wind shear faster or stop reporting an old
value during a long descent.

A wave flight with wind increasing from about 6 m/s below 1,000 m to more than
22 m/s above 5,000 m did not show a useful gain. Larger settings mainly reduced
the number of reported values. Measured airspeed already follows shear. Without
airspeed, the added uncertainty removed too much coverage. The experiment was
not kept.

## Limits

- The recorded wind and vario are references, not ground truth.
- All recordings come from LXNAV instruments.
- The engine state is not available as a live input. Estimates can be wrong
  while an engine runs.
- Flying mass is not recorded. Netto uses the polar's reference mass.
- Without airspeed, straight flight cannot reveal the crosswind component.
- The four wave flights improve coverage of wind shear, but more aircraft and
  instrument families would make the conclusions stronger.
- Most source recordings are not repository fixtures. The public flight links,
  method, and two committed recordings provide reviewable anchors.

## Per-flight results with measured airspeed

The table contains the 16 recordings available for the current replay. “Wave”
marks the four flights used to check long straight flight and wind shear.

### Wave flights

| Flight | Glider | Instrument | Vario RMS | Wind speed RMS | Wind direction RMS |
| --- | --- | --- | ---: | ---: | ---: |
| [537399](https://www.weglide.org/flight/537399) | Duo Discus XLT | LX9000F | 0.46 m/s | 3.14 m/s | 36.74° |
| [658209](https://www.weglide.org/flight/658209) | DG-800B | LX9000 | 0.25 m/s | 2.27 m/s | 18.87° |
| [1015312](https://www.weglide.org/flight/1015312) | JS-3-18m | LX9070 | 0.38 m/s | 3.14 m/s | 11.70° |
| [1023558](https://www.weglide.org/flight/1023558) | JS-3-18m | LX9070 | 0.33 m/s | 6.21 m/s | 21.24° |

### Other soaring flights

| Flight | Glider | Instrument | Vario RMS | Wind speed RMS | Wind direction RMS |
| --- | --- | --- | ---: | ---: | ---: |
| [1113539](https://www.weglide.org/flight/1113539) | LS 1 | S10 | 0.48 m/s | 1.30 m/s | 20.98° |
| [1120273](https://www.weglide.org/flight/1120273) | LS 1 | S10 | 0.53 m/s | 1.57 m/s | 13.53° |
| [1138165](https://www.weglide.org/flight/1138165) | ASH 25m | LX9000 | 0.44 m/s | 1.14 m/s | 24.13° |
| [1140266](https://www.weglide.org/flight/1140266) | ASH 25m | LX9000 | 0.51 m/s | 1.13 m/s | 14.83° |
| [1141558](https://www.weglide.org/flight/1141558) | JS-3-18m | LX9070 | 0.47 m/s | 1.73 m/s | 17.16° |
| [1153141](https://www.weglide.org/flight/1153141) | ASH 26e | LX9070 | 0.25 m/s | 2.32 m/s | 43.95° |
| [1168132](https://www.weglide.org/flight/1168132) | ASW 27B | S100 | 0.92 m/s | 1.24 m/s | 27.67° |
| [1173566](https://www.weglide.org/flight/1173566) | ASW 27B | S100 | 0.92 m/s | 1.45 m/s | 27.76° |
| [1179475](https://www.weglide.org/flight/1179475) | JS-3-18m | LX9070 | 0.51 m/s | 1.90 m/s | 52.88° |
| [1179605](https://www.weglide.org/flight/1179605) | ASH 26e | LX9070 | 0.29 m/s | 2.40 m/s | 72.34° |
| [1184098](https://www.weglide.org/flight/1184098) | ASW 27B | S100 | 0.86 m/s | 1.99 m/s | 54.01° |
| [1188417](https://www.weglide.org/flight/1188417) | JS-3-18m | LX9070 | 0.72 m/s | 2.47 m/s | 22.51° |

## Per-flight results without measured airspeed

The sample count is the number of recorded wind values for which Updraft also
reported wind. It makes the limited wave-flight coverage visible.

| Flight | Vario RMS | Wind samples | Wind speed RMS | Wind direction RMS |
| --- | ---: | ---: | ---: | ---: |
| [537399](https://www.weglide.org/flight/537399) | 0.95 m/s | 81 | 6.15 m/s | 37.71° |
| [658209](https://www.weglide.org/flight/658209) | 1.09 m/s | 4,483 | 4.47 m/s | 35.84° |
| [1015312](https://www.weglide.org/flight/1015312) | 0.59 m/s | 848 | 3.94 m/s | 14.69° |
| [1023558](https://www.weglide.org/flight/1023558) | 0.78 m/s | 1,921 | 9.83 m/s | 21.37° |
| [1113539](https://www.weglide.org/flight/1113539) | 0.62 m/s | 18,859 | 1.49 m/s | 23.14° |
| [1120273](https://www.weglide.org/flight/1120273) | 0.69 m/s | 15,697 | 1.42 m/s | 16.38° |
| [1138165](https://www.weglide.org/flight/1138165) | 0.65 m/s | 18,107 | 1.51 m/s | 29.89° |
| [1140266](https://www.weglide.org/flight/1140266) | 0.70 m/s | 15,778 | 1.65 m/s | 17.16° |
| [1141558](https://www.weglide.org/flight/1141558) | 0.83 m/s | 12,636 | 1.93 m/s | 17.35° |
| [1153141](https://www.weglide.org/flight/1153141) | 0.80 m/s | 15,476 | 2.44 m/s | 52.08° |
| [1168132](https://www.weglide.org/flight/1168132) | 1.30 m/s | 25,463 | 1.48 m/s | 29.44° |
| [1173566](https://www.weglide.org/flight/1173566) | 1.32 m/s | 17,781 | 1.58 m/s | 30.99° |
| [1179475](https://www.weglide.org/flight/1179475) | 1.06 m/s | 24,362 | 2.42 m/s | 67.83° |
| [1179605](https://www.weglide.org/flight/1179605) | 1.00 m/s | 25,196 | 3.02 m/s | 77.80° |
| [1184098](https://www.weglide.org/flight/1184098) | 1.21 m/s | 13,590 | 2.21 m/s | 67.26° |
| [1188417](https://www.weglide.org/flight/1188417) | 1.16 m/s | 10,569 | 3.38 m/s | 31.37° |

## Recorded sink rate by turn direction

The values are `recorded netto − recorded vario`. Right and left turns use
25° to 70° of bank. Wings-level values use at most 5° of bank.

### Wave flights

| Flight | Glider | Right turns | Left turns | Wings level | Right minus left |
| --- | --- | ---: | ---: | ---: | ---: |
| [658209](https://www.weglide.org/flight/658209) | DG-800B | +1.80 m/s | +0.51 m/s | +1.05 m/s | +1.28 m/s |
| [1015312](https://www.weglide.org/flight/1015312) | JS-3-18m | +1.56 m/s | +0.31 m/s | +1.15 m/s | +1.25 m/s |
| [1023558](https://www.weglide.org/flight/1023558) | JS-3-18m | +1.27 m/s | +3.79 m/s | +1.21 m/s | −2.52 m/s |

### Other soaring flights

| Flight | Glider | Right turns | Left turns | Wings level | Right minus left |
| --- | --- | ---: | ---: | ---: | ---: |
| [1113539](https://www.weglide.org/flight/1113539) | LS 1 | +1.04 m/s | +1.17 m/s | +1.17 m/s | −0.13 m/s |
| [1120273](https://www.weglide.org/flight/1120273) | LS 1 | +1.06 m/s | +1.18 m/s | +1.23 m/s | −0.12 m/s |
| [1138165](https://www.weglide.org/flight/1138165) | ASH 25m | +1.29 m/s | +1.42 m/s | +0.74 m/s | −0.13 m/s |
| [1140266](https://www.weglide.org/flight/1140266) | ASH 25m | +0.98 m/s | +0.93 m/s | +0.67 m/s | +0.05 m/s |
| [1141558](https://www.weglide.org/flight/1141558) | JS-3-18m | +1.93 m/s | −0.69 m/s | +0.83 m/s | +2.63 m/s |
| [1153141](https://www.weglide.org/flight/1153141) | ASH 26e | +1.02 m/s | +2.38 m/s | +0.83 m/s | −1.36 m/s |
| [1168132](https://www.weglide.org/flight/1168132) | ASW 27B | +1.52 m/s | +0.78 m/s | +0.98 m/s | +0.74 m/s |
| [1173566](https://www.weglide.org/flight/1173566) | ASW 27B | +2.18 m/s | +0.92 m/s | +1.08 m/s | +1.27 m/s |
| [1179475](https://www.weglide.org/flight/1179475) | JS-3-18m | +1.71 m/s | −0.58 m/s | +0.99 m/s | +2.29 m/s |
| [1179605](https://www.weglide.org/flight/1179605) | ASH 26e | −0.30 m/s | +2.83 m/s | +0.91 m/s | −3.12 m/s |
| [1184098](https://www.weglide.org/flight/1184098) | ASW 27B | +1.52 m/s | +1.09 m/s | +1.05 m/s | +0.43 m/s |
| [1188417](https://www.weglide.org/flight/1188417) | JS-3-18m | +1.38 m/s | +1.09 m/s | +0.89 m/s | +0.30 m/s |

Flight 537399 has no recorded netto and is absent from this table.

# Vertical speed, netto and wind from recorded flight data

Investigation of "how close can Updraft get to an LXNAV vario, with only GNSS,
pressure altitude and true airspeed?". The answer is the `updraft_air` crate.

Measured against fourteen recordings from seven aircraft, six glider types and
five LXNAV instrument models. Only `testdata/weglide_1141558.igc` is in the
repository. The others are the WeGlide flights that the table below links to,
and the numbers taken from them cannot be reproduced from this repository
alone.

Flight [1191252](https://www.weglide.org/flight/1191252) came later, from a
PowerFlarm instead of an LXNAV instrument. It carries no reference values, so
it is absent from the table below. It measures the sensors and the ingestion
instead: see
[a PowerFlarm recording, in two encodings](#a-powerflarm-recording-in-two-encodings).

**Headline: vertical speed and wind reach the accuracy of the recorded
instrument values while circling. The netto disagrees, and the recorded netto
is the side that fails a physical check: it changes with the direction of
turn, and three flights record a negative sink rate.**

| Flight | Glider | Registration | Instrument | Vertical speed | Netto | Wind speed | Wind direction |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [1141558](https://www.weglide.org/flight/1141558) | JS-3-18m | D-KPWZ | LX9070 | 0.47 m/s | 1.23 m/s | 1.73 m/s | 17.0° |
| [1179475](https://www.weglide.org/flight/1179475) | JS-3-18m | D-KPWZ | LX9070 | 0.51 m/s | 1.35 m/s | 1.92 m/s | 53.2° |
| [1188417](https://www.weglide.org/flight/1188417) | JS-3-18m | OK-3314 | LX9070 | 0.72 m/s | 1.15 m/s | 2.52 m/s | 22.2° |
| [1153141](https://www.weglide.org/flight/1153141) | ASH 26e | D-KAFE | LX9070 | 0.25 m/s | 1.15 m/s | 2.33 m/s | 43.9° |
| [1179605](https://www.weglide.org/flight/1179605) | ASH 26e | D-KAFE | LX9070 | 0.29 m/s | 1.20 m/s | 2.40 m/s | 72.3° |
| [1138165](https://www.weglide.org/flight/1138165) | ASH 25m | HB-2393 | LX9000 | 0.44 m/s | 0.85 m/s | 1.17 m/s | 24.3° |
| [1140266](https://www.weglide.org/flight/1140266) | ASH 25m | HB-2393 | LX9000 | 0.51 m/s | 0.82 m/s | 1.15 m/s | 14.9° |
| [1168132](https://www.weglide.org/flight/1168132) | ASW 27B | D-6897 | S100 | 0.92 m/s | 0.89 m/s | 1.24 m/s | 27.6° |
| [1173566](https://www.weglide.org/flight/1173566) | ASW 27B | D-6897 | S100 | 0.92 m/s | 0.97 m/s | 1.43 m/s | 27.8° |
| [1184098](https://www.weglide.org/flight/1184098) | ASW 27B | D-6897 | S100 | 0.86 m/s | 0.96 m/s | 2.00 m/s | 53.4° |
| [1113539](https://www.weglide.org/flight/1113539) | LS 1 | D-9486 | S10 | 0.48 m/s | 0.84 m/s | 1.31 m/s | 21.0° |
| [1120273](https://www.weglide.org/flight/1120273) | LS 1 | D-9486 | S10 | 0.53 m/s | 0.91 m/s | 1.57 m/s | 13.6° |
| [1131653](https://www.weglide.org/flight/1131653) | LS 1 | D-9486 | S10 | 0.50 m/s | 0.89 m/s | 1.43 m/s | 57.7° |
| [1174605](https://www.weglide.org/flight/1174605) | Duo Discus XLT | D-KBBQ | LX9000F | 0.52 m/s | — | — | — |

All values are RMS differences against the recorded values, over soaring
flight. Flight 1174605 records no netto, and its 64 wind records are too few
to score. Wind direction scales with wind strength: the flights with a large
error are the weak-wind days, where the recorded wind itself drops below
2 m/s for hours.

Every recording has an LX HAWK inertial platform, versions 14 and 16, so
nothing below is explained by its presence or absence.

**The three S100 flights show what an RMS difference does not.** Their 0.85 to
0.92 m/s is the worst vertical speed in the table, and their averaged climb
rate is the best: 0.02 to 0.05 m/s over a thermal. The S100's own vario moves
twice as fast as the others, 0.68 to 0.72 m/s away from its own 5 second
average against 0.20 to 0.41 m/s everywhere else, the S10 included. That is
the instrument, not the family: a fast S100 needle and a damped S10 needle
have to be matched by the same estimate. See
[Reading the error figures](#reading-the-error-figures).

**The launch is excluded, and has to be.** Five of these gliders have an
engine. While it runs, the estimate and the recorded values both go wrong: on
flight 1138165 the first five minutes have the instrument reporting a 32 m/s
wind and the estimate 22 m/s. Excluding the engine and ground phase, 2% to 7%
of each recording, moves that flight's netto from 2.23 to 0.85 m/s and its
wind from 7.22 to 1.71 m/s of vector RMS, and leaves the rest almost
unchanged. Every hour of both ASH 25m flights after the engine stops scores
between 1.2 and 2.2 m/s of vector RMS.

Nothing in the live inputs says the engine is running, so the estimate cannot
exclude it by itself. Rejecting outlying airspeed measurements does not help
and makes things much worse: the wind state drifts while the measurements are
rejected, which makes the next ones look like outliers too, and the filter
never recovers. Vector RMS went from 2.66 to 22.10 m/s on one flight. A
consumer that knows the engine is running should discard the estimate while
it is.

The test `libs/updraft_air/tests/recorded_flight.rs` recomputes the 1141558 row
and keeps it in an Insta snapshot.

## Reading the error figures

An RMS difference is not the error a pilot sees. It squares each difference
before averaging, so a few large moments set the value while most samples sit
well below it. For the vertical speed the median difference is 0.16 to 0.35 m/s
against an RMS of 0.26 to 0.72 m/s, and 90% of samples are inside 0.44 to
1.13 m/s.

Three further measurements say what the figure is made of.

**Most of it is bandwidth, not error.** The recorded vario differs from its own
5 second average by 0.20 to 0.72 m/s, depending on the instrument, which is as
large as the whole difference against the estimate. That is the instrument's
own second-to-second movement. A recording of one sample per second cannot
contain it, so no estimate built from that recording can reproduce it.

**It averages away.** RMS of the difference between running averages of both
signals, by window:

| Flight | Instrument | 1 s | 5 s | 15 s | 30 s | 60 s | Per climb |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1173566 | S100 | 0.91 | 0.58 | 0.25 | 0.15 | 0.10 | 0.022 |
| 1168132 | S100 | 0.92 | 0.57 | 0.23 | 0.13 | 0.08 | 0.052 |
| 1184098 | S100 | 0.86 | 0.52 | 0.20 | 0.11 | 0.06 | 0.020 |
| 1188417 | LX9070 | 0.72 | 0.62 | 0.35 | 0.22 | 0.15 | 0.101 |
| 1179475 | LX9070 | 0.51 | 0.41 | 0.28 | 0.20 | 0.15 | 0.083 |
| 1141558 | LX9070 | 0.46 | 0.35 | 0.19 | 0.13 | 0.09 | 0.034 |
| 1179605 | LX9070 | 0.29 | 0.20 | 0.11 | 0.08 | 0.07 | 0.083 |
| 1153141 | LX9070 | 0.25 | 0.17 | 0.09 | 0.06 | 0.05 | 0.038 |
| 1120273 | S10 | 0.53 | 0.46 | 0.34 | 0.24 | 0.16 | 0.068 |
| 1131653 | S10 | 0.50 | 0.42 | 0.31 | 0.22 | 0.15 | 0.045 |
| 1113539 | S10 | 0.48 | 0.41 | 0.31 | 0.23 | 0.15 | 0.060 |
| 1140266 | LX9000 | 0.51 | 0.44 | 0.26 | 0.16 | 0.10 | 0.064 |
| 1138165 | LX9000 | 0.44 | 0.37 | 0.22 | 0.14 | 0.09 | 0.036 |
| 1174605 | LX9000F | 0.51 | 0.32 | 0.14 | 0.08 | 0.06 | 0.011 |

An error that averages away is a difference in filtering, not a wrong reading.
An error that stays is a bias. The last column is the difference in the average
climb rate over a whole thermal, which is the number a pilot decides on.

The S100 rows make the point on their own. They start worst and end best. Over
the 350 climbs in all fourteen recordings that hold 120 seconds or more of
circling, the averaged climb rate differs from the instrument's by 0.05 m/s on
average, 0.07 m/s RMS, and never by more than 0.29 m/s.

The netto looks worse: 0.84 to 1.35 m/s at one second, and still 0.41 to
0.92 m/s over 60 seconds on the flights with an asymmetric reference. That is the recorded netto's turn offset (see below),
which is a bias in the reference and does not average away. Over stretches of
60 seconds or more with the wings level, where the offset is absent, the
averaged netto differs by 0.13 to 0.18 m/s.

## What the recordings contain

The `I` record defines FXA, ENL, TAS, GSP, TRT, VAT, OAT and ACZ everywhere,
plus NET, AOR and AOP on everything except the LX9000F, SIU on everything
except the S10 and S100, and AOA on the LX9070 only. The `J` record
defines WDI and either WSP or WVE, depending on the firmware; both hold the
same quantity. LXNAV writes speeds as hundredths of a kilometre per hour,
vertical speeds as hundredths of a metre per second, accelerations as
hundredths of *g*, and roll and pitch in whole degrees.

Two independent checks confirm the field offsets. ACZ reads 1.00 in level
flight, and ACZ matches `1/cos(AOR)` across a flight.

## The algorithm

**Vertical speed.** The total energy height `h + v²/2g` removes the height that
the glider trades against airspeed. Its derivative is the total-energy vertical
speed. Two exponential stages with a 2 s time constant smooth it.

The compensation is what makes this work. Against the recorded vario, the
unsmoothed pressure-altitude rate correlates 0.73. The two-stage filter on the
total energy height correlates 0.96. An optimal linear filter, fitted on the
whole flight, reaches 0.965.

`h` is not the pressure altitude alone. See [Both altitudes are
useful](#both-altitudes-are-useful) below.

**Wind.** Each sample states that `TAS = ‖ground velocity − wind‖`. That is one
scalar measurement of a two-component state, so an extended Kalman filter
tracks the wind vector. The measurement constrains the wind along the current
heading only. A turn is what makes the wind observable, and one full circle is
enough to converge.

This one filter replaces the usual split between a circling method and a
straight-flight method. In a circle the heading sweeps every direction, so the
filter behaves like a circle fit in velocity space. In a glide it still
corrects the along-heading component, which is why the estimate does not go
stale between thermals. On flight 1141558, a circle fit that holds its last
value between circles scores 3.14 m/s vector RMS against the recorded wind; the
filter scores 2.77 m/s.

The GNSS fix accuracy scales the measurement variance. The wind is reported
only after the filter converges, which took 118 s on flight 1141558.

**Netto.** Subtracting the wind from the ground velocity gives the air-relative
heading. Its rate of change is the turn rate, which fixes the bank angle and
the load factor `n = √(1 + (ω·v/g)²)`. The derived bank angle correlates 0.982
with the recorded AOR, which confirms the wind estimate independently.

A glide polar is quoted as equivalent airspeed against sink rate at sea level.
Both axes scale with `1/√σ` at a density ratio `σ`, and both scale with `√n` in
a turn, so the polar is read at `v·√σ/√n` and its result is scaled back. The
netto is the vertical speed plus that sink rate.

## Both altitudes are useful

An earlier version of this investigation stated that the GNSS altitude is "too
noisy to differentiate". That was wrong.

Sample-to-sample, the GNSS altitude of a uBLOX NEO-M8Q is *less* noisy than the
logged pressure altitude: 0.47 to 0.60 m against 0.57 to 0.82 m. The metre
resolution of the IGC pressure altitude is not the reason. Through the vario
filter, one metre of rounding is worth only 0.049 m/s of vertical speed, while
the difference against the recorded vario is 0.3 to 0.9 m/s.

The two altitudes cannot be averaged directly, because they differ by the
altimeter setting, the geoid, and the deviation from the ISA temperature. A
complementary filter with a 5 s crossover tracks that difference as an offset,
which puts the slow height changes on the GNSS altitude and the fast ones on
the pressure altitude. That is worth about 10% of the vertical-speed error.

One receiver breaks the scheme. The uBLOX LEA-4S in the LX9000F reports its
altitude a second late: its height rate correlates 0.927 with the pressure
height rate at the same sample, and 0.993 one sample back. Averaging in a
delayed copy of the same signal made that flight 32% worse. Every NEO-M8Q and
NEO-M9N recording correlates 0.99 or better at zero lag.

The filter therefore compares the GNSS height rate against the current and the
previous pressure height rate, and stops using the GNSS altitude while the
previous one fits better. With that check:

| Flight | Receiver | Pressure only | Both altitudes | GNSS used |
| --- | --- | --- | --- | --- |
| 1174605 | LEA-4S | 0.53 m/s | 0.53 m/s | 0.4% |
| 1188417 | NEO-M9N | 0.87 m/s | 0.71 m/s | 97% |
| 1179475 | NEO-M8Q | 0.59 m/s | 0.51 m/s | 100% |
| 1179605 | NEO-M8Q | 0.32 m/s | 0.31 m/s | 98% |
| 1153141 | NEO-M8Q | 0.30 m/s | 0.29 m/s | 100% |
| 1141558 | NEO-M8Q | 0.53 m/s | 0.48 m/s | 100% |

No flight is worse, the mean improves from 0.52 to 0.47 m/s, and the lagging
receiver falls back to the pressure altitude on its own. Gating on FXA or SIU
in addition changes nothing, so the estimate does not do it.

The **position** is a different case. Ground velocity derived from consecutive
positions matches the recorded track and ground speed closely enough that the
wind results are the same on all six flights, to within 0.1 m/s of vector RMS.
So the position is not needed *when the source reports track and ground speed*.
It is the natural substitute when the source does not.

## The recorded netto depends on the direction of turn

The recorded netto minus the recorded vario is the sink rate that the
instrument applied. Split by the recorded roll angle:

| Flight | Glider | Instrument | Right turns | Left turns | Wings level | Right − left |
| --- | --- | --- | --- | --- | --- | --- |
| [1140266](https://www.weglide.org/flight/1140266) | ASH 25m, HB-2393 | LX9000 | +0.98 m/s  0.51 m/s | 0.82 m/s | 1.15 m/s | 14.9° |
| [1138165](https://www.weglide.org/flight/1138165) | ASH 25m, HB-2393 | LX9000 | +1.30 m/s  0.44 m/s | 0.85 m/s | 1.17 m/s | 24.3° |
| [1120273](https://www.weglide.org/flight/1120273) | LS 1, D-9486 | S10 | +1.06 m/s  0.53 m/s | 0.91 m/s | 1.57 m/s | 13.6° |
| [1113539](https://www.weglide.org/flight/1113539) | LS 1, D-9486 | S10 | +1.04 m/s  0.48 m/s | 0.84 m/s | 1.31 m/s | 21.0° |
| [1131653](https://www.weglide.org/flight/1131653) | LS 1, D-9486 | S10 | +1.17 m/s | +1.38 m/s | +1.35 m/s | −0.21 |
| [1188417](https://www.weglide.org/flight/1188417) | JS-3-18m, OK-3314 | LX9070 | +1.39 m/s  0.72 m/s | 1.15 m/s | 2.52 m/s | 22.2° |
| [1184098](https://www.weglide.org/flight/1184098) | ASW 27B, D-6897 | S100 | +1.53 m/s  0.86 m/s | 0.96 m/s | 2.00 m/s | 53.4° |
| [1168132](https://www.weglide.org/flight/1168132) | ASW 27B, D-6897 | S100 | +1.54 m/s  0.92 m/s | 0.89 m/s | 1.24 m/s | 27.6° |
| [1173566](https://www.weglide.org/flight/1173566) | ASW 27B, D-6897 | S100 | +2.19 m/s  0.92 m/s | 0.97 m/s | 1.43 m/s | 27.8° |
| [1153141](https://www.weglide.org/flight/1153141) | ASH 26e, D-KAFE | LX9070 | +1.01 m/s  0.25 m/s | 1.15 m/s | 2.33 m/s | 43.9° |
| [1179475](https://www.weglide.org/flight/1179475) | JS-3-18m, D-KPWZ | LX9070 | +1.71 m/s  0.51 m/s | 1.35 m/s | 1.92 m/s | 53.2° |
| [1141558](https://www.weglide.org/flight/1141558) | JS-3-18m, D-KPWZ | LX9070 | +1.94 m/s  0.47 m/s | 1.23 m/s | 1.73 m/s | 17.0° |
| [1179605](https://www.weglide.org/flight/1179605) | ASH 26e, D-KAFE | LX9070 | −0.33 m/s  0.29 m/s | 1.20 m/s | 2.40 m/s | 72.3° |

Turns are samples with 25° to 70° of bank. A sink rate can never be negative,
and it cannot depend on which way the glider turns. Three flights record a
negative one: −0.59 and −0.70 m/s in left turns, and −0.33 m/s in right turns.
The recorded netto there says the glider climbed through the air while it
circled, which no glider does.

The asymmetry follows the **aircraft**, not the firmware or the instrument.
Every aircraft flown more than once keeps the sign of its own asymmetry:
HB-2393 is symmetric on both days, D-9486 slightly high to the left on all
three, D-6897 high to the right on all three, D-KPWZ high to the right on
both, D-KAFE high to the left on both. Four instrument models are
represented, and both the cleanest and the worst aircraft carry the same
LX9070 generation, so the instrument does not explain it. That points at the
pressure-port installation or a steady sideslip.

**The netto difference ranks with the asymmetry.** Averaged per aircraft, how
asymmetric the instrument's own netto is orders them almost exactly as the
difference against the estimate:

| Aircraft | Glider | Flights | Asymmetry | Netto RMS |
| --- | --- | --- | --- | --- |
| HB-2393 | ASH 25m | 2 | 0.07 | 0.86 m/s |
| D-9486 | LS 1 | 3 | 0.15 | 0.89 m/s |
| OK-3314 | JS-3-18m | 1 | 0.30 | 1.17 m/s |
| D-6897 | ASW 27B | 3 | 0.82 | 0.93 m/s |
| D-KAFE | ASH 26e | 2 | 2.28 | 1.21 m/s |
| D-KPWZ | JS-3-18m | 2 | 2.47 | 1.31 m/s |

Spearman rank correlation 0.94 over the six aircraft, and 0.81 over the
thirteen flights. The estimate agrees best with the aircraft whose recorded
netto is self-consistent, and worst with the aircraft whose recorded netto
contradicts itself between left and right turns. That is what a reference
problem looks like, not a model problem. D-6897 is the one that does not fit:
a middling asymmetry with a good netto difference.

Wings level, where the artifact is absent, the recorded sink rate lands between
0.67 and 1.08 m/s on every flight, which is the range the polars of these five
glider types predict. On flight 1141558, speed bins of 10 km/h agree within
±0.22 m/s with no trend against speed. The netto model is therefore sound, and
the 0.8 to 1.4 m/s RMS in the first table measures the instrument more than the
estimate.

Scoring the netto only where the artifact is absent shows what the estimate is
worth: 0.88 to 1.18 m/s RMS wings level, with a bias of +0.05 to +0.08 m/s,
against 1.38 to 2.21 m/s in turns.

### Which netto is closer to the air

The estimate cannot produce this artifact. The load factor comes from the
square of the turn rate, so the sink rate it applies is the same to the left
and to the right. The recorded netto changes sign with the turn on some
aircraft, and goes negative on three flights, so it carries something that is
not the air.

That does not make the estimate the better number. Neither value was measured
against the true vertical motion of the air, so "better" cannot be scored from
these recordings. The estimate has its own errors: it uses the polar's
reference mass, and it reads the density from the ISA rather than from an
outside air temperature.

What the recordings do establish is narrower and enough: the netto difference
in the first table is not evidence that the estimate is wrong. It ranks with
the asymmetry of the reference, and it drops to the wings-level figure once
the artifact is removed.

## Without an airspeed sensor

An Android device and a PowerFlarm both report position and pressure altitude
but no airspeed. The wind then has to come from the shape of a circle alone: the
ground velocities of one circle lie on a circle in velocity space whose centre
is the wind and whose radius is the airspeed. Fitting all three, instead of
holding the radius at a measured airspeed, is what costs accuracy. Once the wind
is known, the airspeed follows as `‖ground velocity − wind‖`, which restores
part of the total energy compensation.

| Flight | Wind (vec / dir) | Airspeed | Vario: sensor / derived / none |
| --- | --- | --- | --- |
| 1141558 | 3.52 m/s / 20° | 2.47 m/s | 0.46 / 0.95 / 1.26 m/s |
| 1179475 | 3.91 m/s / 73° | 2.92 m/s | 0.51 / 1.32 / 1.88 m/s |
| 1188417 | 5.35 m/s / 35° | 3.73 m/s | 0.71 / 1.26 / 1.93 m/s |
| 1153141 | 4.45 m/s / 48° | 2.20 m/s | 0.25 / 0.86 / 1.49 m/s |
| 1179605 | 5.17 m/s / 87° | 3.45 m/s | 0.29 / 1.09 / 1.55 m/s |
| 1174605 | — | 2.35 m/s | 0.52 / 1.09 / 1.51 m/s |

The wind is 25% to 40% worse in vector RMS than with an airspeed sensor, and it
only updates while circling. The vario roughly doubles its error, but the
derived airspeed still recovers about half the gap to an uncompensated vario.
The netto barely changes, because its error is dominated by the recorded
reference and not by the estimate.

The per-flight figures in this section were measured before the circling sink
rate was corrected to the load factor's `n^1.5` law, and were not re-derived.
The table at the top of this record was.

`updraft_air` implements this path. The recorded-flight test measures both,
and the estimate falls back on its own when a sample carries no airspeed.
Nothing chooses between them ahead of time, so an airspeed source that
connects or drops out mid-flight is handled where it happens.

## Sample rate

Re-running the estimate on decimated recordings shows that 1 Hz is already
past the point where the sample rate matters. Vertical-speed RMS against the
recorded vario, by sample interval:

| Flight | 1 s | 2 s | 3 s | 5 s | 10 s |
| --- | --- | --- | --- | --- | --- |
| 1141558 | 0.48 | 0.50 | 0.52 | 0.58 | 0.66 |
| 1179475 | 0.51 | 0.58 | 0.60 | 0.64 | 0.86 |
| 1188417 | 0.71 | 0.77 | 0.81 | 0.88 | 0.75 |
| 1153141 | 0.29 | 0.29 | 0.27 | 0.37 | 0.73 |
| 1179605 | 0.31 | 0.31 | 0.29 | 0.38 | 0.76 |
| 1174605 | 0.53 | 0.51 | 0.68 | 0.80 | 1.17 |

The 2 s smoothing, not the sample rate, sets the bandwidth. A faster pressure
sensor therefore cannot improve the agreement with a recorded vario. What it
buys is a shorter time constant at the same noise. For white altitude noise of
1 m standard deviation, the vario noise the filter passes through is:

| Time constant | 1 Hz | 5 Hz | 10 Hz | 25 Hz |
| --- | --- | --- | --- | --- |
| 1.0 s | 0.444 | 0.222 | 0.159 | 0.100 |
| 1.5 s | 0.258 | 0.122 | 0.085 | 0.054 |
| 2.0 s | 0.171 | 0.079 | 0.056 | 0.035 |
| 3.0 s | 0.095 | 0.043 | 0.030 | 0.019 |

At 10 Hz, a 1 s time constant is quieter than 2 s at 1 Hz. The two smoothing
stages delay the reading by about `2·τ`, so that halves the vario's lag from
roughly 4 s to 2 s. For centring a thermal that matters more than any of the
error figures above.

That table assumes white noise. A measured device barometer turned out not to
have any, and the conclusion changed: see
[the rate is not what buys the shorter time constant](#the-rate-is-not-what-buys-the-shorter-time-constant).

#### The 2 s default is an optimum, not a safe choice

Sweeping the time constant over flight 1141558 shows what the 4 s of lag buys.
"Step" is the RMS change between one reading and the next, which is how much
the needle moves each second. "Climb error" is the mean error over 60 s blocks,
which is how wrong the strength of a thermal reads.

| Time constant | RMS | Correlation | Step | Climb error |
| --- | --- | --- | --- | --- |
| 0.25 s | 2.083 | 0.629 | 2.554 | 0.087 |
| 0.50 s | 1.681 | 0.720 | 1.911 | 0.079 |
| 1.00 s | 0.985 | 0.874 | 1.040 | 0.060 |
| 1.50 s | 0.608 | 0.942 | 0.667 | 0.051 |
| 2.00 s | 0.467 | 0.963 | 0.474 | 0.056 |
| 3.00 s | 0.585 | 0.944 | 0.284 | 0.088 |
| 4.00 s | 0.771 | 0.900 | 0.194 | 0.124 |

The RMS has a minimum at 2 s, and the two sides of that minimum have different
causes. Below 2 s the step column rises: the reading gets noisier. Above 2 s
the step column keeps falling while the RMS rises, so lag is what costs there.
A 0.25 s time constant on this source moves the needle by 2.55 m/s from one
second to the next, which no pilot can read.

So 4 s of lag is the price of a 1 Hz pressure altitude that carries 0.6 m of
noise. It is not a margin that a different default could remove. A barometer
that carries 0.024 m reaches the same vario noise at 0.25 s, which is 0.5 s of
lag.

The climb error column stays between 0.05 and 0.12 m/s across the whole sweep.
Lag and noise decide whether a pilot can centre a thermal. They barely change
how strong that thermal reads.

`updraft_air` defaults to the 2 s that a logged pressure altitude needs, and
takes another value from the caller, which is the side that knows what sensor
is feeding it.

## Recordings that would settle the time constant

Today the vertical speed carries 0.10 to 0.14 m/s of altitude noise, out of a
0.48 m/s total difference against the instrument. That is the budget a shorter
time constant has to stay inside. Given a sensor's noise and rate, the table
below is the shortest time constant that stays at or below 0.12 m/s:

| Sensor noise | 10 Hz | 25 Hz | 50 Hz |
| --- | --- | --- | --- |
| 0.1 m | 0.30 s | 0.20 s | 0.20 s |
| 0.2 m | 0.45 s | 0.35 s | 0.25 s |
| 0.3 m | 0.55 s | 0.40 s | 0.35 s |
| 0.5 m | 0.80 s | 0.60 s | 0.45 s |
| 0.8 m | 1.05 s | 0.80 s | 0.65 s |

Every row is a large win. Even the worst, 0.8 m of noise at 10 Hz, halves the
lag from about 4 s to 2 s, and a quiet sensor at 25 Hz takes it under half a
second. So the only measurement the decision needs is the noise and the true
rate of a real device barometer.

### A bench recording decides it

Put the device on a table indoors and log the barometer at the fastest rate
the platform gives, for ten minutes, without touching it. Repeat on two or
three devices, because the barometer chip differs between them.

From that recording:

- **The delivered rate.** `SENSOR_DELAY_FASTEST` is a request, not a promise.
  The gaps between event timestamps say what arrived.
- **The noise.** The standard deviation of the second difference, divided by
  `√6`, is the per-sample noise for a white source. That value picks the row
  in the table above.
- **Whether the sensor is already smoothed.** This is the trap that the
  uBLOX LEA-4S sprang on the GNSS altitude. A driver that low-pass filters
  reports a high rate that carries no extra information, and a shorter time
  constant would then buy lag, not bandwidth. It shows as noise that falls
  away faster than `1/√n` when the samples are averaged in blocks of `n`, and
  as the ratio of the power between 2 and 5 Hz to the power between 0.05 and
  0.1 Hz. See
  [screening a device that exposes one pressure sensor](#screening-a-device-that-exposes-one-pressure-sensor)
  for the values that separate a usable source from a filtered one.
- **The resolution.** A coarse quantisation step sets a floor under the noise.

Ten minutes of a still device answers all four. No flight is needed.

The two bench recordings below are not in the repository. Ask the repository
owner for them. A shortened or decimated copy will not do: the screening
indicator reads a band up to 5 Hz, and the cross-spectrum needs the full ten
minutes to average six windows of 4,096 samples.

#### Galaxy S23 result

The first bench recording came from a Samsung Galaxy S23 (`SM-S911B`) with
Android 16. Android
identified its barometer as an STMicro LPS22HH. The phone stayed on a table
indoors and received USB power. A temporary app requested
`SENSOR_DELAY_FASTEST` and saved each pressure value with its
`SensorEvent.timestamp`.

The sensor delivered 15,002 samples in 600.04 s. The mean rate was exactly
25.000 Hz. The median gap was 40.000 ms, and the largest gap was 40.146 ms.
No gap exceeded 41 ms.

The second-difference method gives 0.00281 hPa of pressure noise. This is
0.0240 m at the mean pressure of 981.26 hPa. The noise was stable between
0.0228 and 0.0248 m in the ten one-minute sections.

The values are not independent. Consecutive altitude differences had a
correlation of -0.110. Independent white samples would give -0.5. The
second-difference noise of eight-sample block means was 0.0334 m. Independent
white samples would reduce it to 0.0085 m. The sensor or its Android driver
therefore smooths the output before the app receives it.

Android reported a resolution of 0.0002 hPa. The recorded values used steps
of 1/4096 hPa, or 0.000244 hPa. This step is 0.00209 m at the mean pressure.

Replaying the recording through the two vertical-speed smoothing stages gives:

| Time constant | Full-recording RMS | Highest one-minute RMS | Approximate lag |
| --- | --- | --- | --- |
| 0.200 s | 0.104 m/s | 0.121 m/s | 0.400 s |
| 0.225 s | 0.092 m/s | 0.110 m/s | 0.450 s |
| 0.250 s | 0.083 m/s | 0.101 m/s | 0.500 s |

A 0.25 s time constant stays below the 0.12 m/s budget with margin on this
device. It reduces the approximate lag from 4 s to 0.5 s. This one recording
does not justify a common Android value. More device models must confirm the
result first.

#### What the smoothing costs

A driver that smooths also delays. The power above 10 Hz is 5% of the power
between 1 and 2 Hz. A first-order low pass with a time constant of 86 ms has
that ratio, and is 3 dB down at 1.9 Hz. A driver of that shape adds about
0.09 s on top of the smoothing stages, so a 0.25 s time constant would read
the air of about 0.59 s ago rather than 0.50 s.

Treat 86 ms as one point in a wide range, not as a measurement. Fitting the
same first-order shape to the recording gives 143 ms over 1 to 12.5 Hz, 223 ms
over 0.2 to 10 Hz, and 848 ms over 0.05 to 12.5 Hz. The fitted value follows
the chosen band, because a stationary phone has no height signal to lag. A
second stream from the same device measures the delay directly, which the LG
G7 below supplies.

#### The rate is not what buys the shorter time constant

Decimating the same recording separates the sample rate from the quality of
the sensor. Vertical-speed noise, in m/s:

| Rate | Altitude noise | τ = 0.25 s | τ = 0.5 s | τ = 1 s | τ = 2 s |
| --- | --- | --- | --- | --- | --- |
| 25 Hz | 0.024 m | 0.083 | 0.039 | 0.018 | 0.009 |
| 5 Hz | 0.047 m | 0.097 | 0.043 | 0.020 | 0.010 |
| 1 Hz | 0.061 m | 0.082 | 0.058 | 0.028 | 0.013 |

**One sample per second from this barometer still supports a 0.25 s time
constant.** The earlier section of this investigation expected the rate to be
what allows a shorter one. For white noise it would be. This sensor's noise is
not white, so decimating it loses less than the white model predicts, and what
is left is still ten times quieter than the 0.6 m of a logged pressure
altitude.

The vertical-speed noise scales with the altitude noise, so the same 25 Hz
recording at the 0.6 m of a flight recorder would give 2.1 m/s at a 0.25 s
time constant. The constant therefore has to follow the noise of the source,
not its rate, and `AirStateEstimator::with_vertical_speed_time_constant` lets
the caller set it. The default stays at the 2 s that a logged pressure
altitude needs.

#### LG G7 public and unfiltered result

The second bench device was an LG G7 ThinQ (`LM-G710`) with Android 10. It
exposes two pressure sensors. The public `android.sensor.pressure` sensor
reports at 25 Hz. The LG-only `lge.sensor.lg_unfiltered_pressure` sensor
reports at 32 Hz.

A temporary app registered both sensors with `SENSOR_DELAY_FASTEST`. It saved
both streams with their `SensorEvent.timestamp` values. The public stream
delivered 15,001 samples in 600.00 s. The unfiltered stream delivered 19,201
samples in the same interval. Neither stream had a missing sample.

The unfiltered stream carries more noise, but it is still well below the
vertical-speed budget:

| Source | Rate | Altitude noise | RMS at τ = 0.25 s |
| --- | --- | --- | --- |
| Public | 25 Hz | 0.000476 m | 0.0119 m/s |
| Unfiltered | 32 Hz | 0.003116 m | 0.0415 m/s |

The simultaneous timestamps expose the filter that LG applies to the public
sensor. The analysis first resampled the unfiltered stream at each public
timestamp. It then split both streams into six overlapping windows of 4,096
samples. Each window had its best-fit line removed and a Hann window applied.
The averaged cross-spectrum gives the gain and phase delay of the public
stream relative to the unfiltered stream:

| Frequency | Public gain | Coherence | Phase delay |
| --- | --- | --- | --- |
| 0.049 Hz | 0.986 | 0.988 | 1.37 s |
| 0.098 Hz | 0.708 | 0.947 | 1.42 s |
| 0.201 Hz | 0.345 | 0.912 | 1.59 s |

The public output is 3 dB down at about 0.1 Hz. At that frequency, the LG
driver adds 1.4 s of phase delay. Two estimator stages with a 0.25 s time
constant add about 0.5 s more. The combined phase delay is therefore about
1.9 s at 0.1 Hz.

The unfiltered sensor supports the same 0.25 s estimator time constant. It
avoids the 1.4 s public-driver delay and stays below the 0.12 m/s noise budget.
Its string type is LG-specific, so a portable Android adapter cannot depend on
it. The public recording looked exceptionally quiet because the driver had
already removed most of its useful bandwidth.

#### Screening a device that exposes one pressure sensor

The LG G7 exposes an unfiltered stream, so its driver filter is measurable.
Most devices expose `android.sensor.pressure` alone. A single stream must
therefore show whether a driver has already removed the bandwidth.

The indicator that the Galaxy S23 section used cannot do that. It compares the
power above 10 Hz against the power between 1 and 2 Hz. The LG driver cuts at
0.1 Hz, a decade below the lower anchor, so both LG streams pass 1 to 2 Hz
almost unchanged. That ratio does not rank the three streams by their measured
filtering:

| Stream | Measured driver filter | P(>10 Hz) / P(1-2 Hz) | P(2-5 Hz) / P(0.05-0.1 Hz) |
| --- | --- | --- | --- |
| Galaxy S23 public | not measurable | 0.0493 | 1.97e-02 |
| LG G7 unfiltered | none applied | 0.0030 | 6.74e-03 |
| LG G7 public | 3 dB down at 0.1 Hz | 0.0155 | 5.32e-05 |

The second ratio anchors its lower band at the frequencies a vario uses. It
puts the three streams in the order that their measured filtering gives. The
filtered LG public stream falls 370 times below the S23. Use that ratio to
screen a new device, and read a low value as a reason to reject the source.

Two other single-stream indicators failed:

- **Correlation between consecutive differences.** The LG public stream gives
  +0.324 and the LG unfiltered stream gives +0.469. A positive value therefore
  does not prove that the driver filters.
- **The fitted roll-off.** The fit reaches its upper bound on the LG public
  stream and moves between 339 and 521 ms on the LG unfiltered stream as the
  band changes. The LG public noise of 0.000476 m is at its quantisation step
  of 1/16384 hPa, or 0.00052 m, so quantisation fills its high band instead of
  sensor output.

#### Noise alone cannot set the time constant

The LG public stream is the quietest source in this investigation, at
0.000476 m, and also the worst. Its noise budget alone would accept a time
constant far below 0.25 s, while its reading is already 1.4 s old. A noise
figure measures what a filter removed, so a driver that removes the signal
improves the figure.

This is the third source here that hides lag behind a good noise figure. The
uBLOX LEA-4S reports its altitude one second late. The Galaxy S23 driver
smooths before the app receives the output. The LG public driver delays by
1.4 s. A source therefore has to qualify on noise and on bandwidth together.

### A flight recording checks the rest

Two things a table cannot show:

- **The static source.** A device barometer measures cabin pressure, which
  moves with the vents, with the airspeed, and with the canopy. That is a
  systematic error, and it decides whether a device barometer may drive the
  height at all when an instrument on the aircraft static port is connected.
- **That a shorter time constant does no harm.** A 1 Hz reference cannot
  confirm the behaviour of a filter faster than about 2 s, so this can only
  rule out a regression, not confirm the gain.

One flight of two hours or more, with the device beside a connected
instrument, covering a launch, several climbs, at least one fast glide, and
vents opened and closed during a steady glide.

### What a recording has to contain

One row per sample, with each source on its own row and its own timestamp.
A single fused row per second cannot express a 25 Hz barometer, which is the
whole point of the recording. An IGC file therefore cannot carry this.

| Source | Fields |
| --- | --- |
| `baro` | time, raw pressure in pascals |
| `fix` | time, latitude, longitude, GNSS altitude, track, ground speed, accuracy |
| `tas` | time, true airspeed, if an instrument is connected |
| `reference` | time, the instrument's own vario and pressure altitude |

Raw pressure rather than a converted altitude, so that the conversion stays in
one place. The instrument's netto is not worth recording: it depends on the
direction of turn (see below).

**The timestamps have to share one clock.** On Android a sensor event is
stamped on a different clock from a location fix. Recording both clocks once
at the start, or every sensor event's own timestamp plus a mapping, is enough.
A 100 ms error between the two sources puts the whole point of the exercise
out of reach: the height filter pairs the two altitudes within 200 ms.

## A better altitude, and the QNH

The height filter throws away the offset between the two altitudes, because
vertical speed only needs the changes. The offset itself is worth keeping: it
holds both the altimeter setting and the temperature of the air below the
glider.

The offset is not constant. Over one flight, `GNSS altitude − pressure
altitude` moves by 10 to 43 m. Two terms explain almost all of that:

```text
true altitude = pressure altitude + a + b · pressure altitude
```

`a` is the altimeter setting, which does not change with height. `b` is how far
the mean temperature of the air below the glider deviates from the ISA, as a
fraction: a warm air column is thicker, so the glider is higher than the
altimeter says. Fitting both over a flight leaves a residual of 3.9 to 6.9 m,
or 2.8 to 4.8 m when `a` is allowed to drift hour by hour. Part of that
residual is the geoid, which moves by several metres across a 200 km flight and
which [`updraft_egm96`] already models.

Both terms check out against an independent measurement:

| Flight | `b` as ΔT | ΔT from recorded OAT | `a` | Offset on the ground | Altitude flown |
| --- | --- | --- | --- | --- | --- |
| 1141558 | +6.7 K | +7.7 K | 150.5 m | 159.0 m | 360–1956 m |
| 1153141 | +11.5 K | +7.8 K | 138.2 m | 138.0 m | 78–2198 m |
| 1174605 | +8.8 K | +9.9 K | 11.9 m | 14.0 m | 522–2287 m |
| 1179475 | +15.5 K | +16.1 K | 104.8 m | 150.0 m | 831–4091 m |
| 1179605 | +15.7 K | +12.9 K | 118.4 m | 157.0 m | 926–3903 m |
| 1188417 | +16.6 K | +14.8 K | 51.6 m | 77.0 m | 696–4080 m |

The temperature term lands within 1 to 4 K of the recorded outside air
temperature on every flight, and the cockpit sensor is not a reference
instrument itself.

The altimeter setting is only as good as the extrapolation to sea level. The
three flights that stayed below 2300 m recover the offset measured before
take-off to within 0.2 to 8.5 m, which is 0.02 to 1 hPa. The three that climbed
to 4000 m miss it by 25 to 45 m, or 3 to 5 hPa, because one straight line
through the whole column cannot follow a temperature profile that changes above
the convective layer. A QNH estimate therefore has to weight the low and recent
part of the flight.

### Extrapolating to sea level does not work

`updraft_air` reports the altitude and an altimeter setting, but not from
this two-term fit. Running the fit online, as a two-state Kalman filter fed one
measurement per fix, recovers the altimeter setting measured before take-off
to within a few metres on the two flights that stayed below 2300 m, and misses
it by 20 to 45 m on the three that climbed to 4000 m. Weighting the low
samples more heavily, and feeding the filter the pre-take-off ground data,
both leave that unchanged. One straight line cannot follow a temperature
profile that changes above the convective layer, and near the ground the two
terms are not separable at all: the glider sits at one altitude, so any split
between them fits the data equally well.

### A QNH belongs to a field, not to the glider

The altimeter setting was first solved from the ratio of two *ISA* pressure
profiles. That is the definition of QNH, and it holds only where the glider
sits. From 3800 m on a warm day the reduction runs down through a column the
ISA does not describe, and the reported setting climbed with the glider:
1022.7 hPa at 700 m to 1042.3 hPa at 3800 m on flight
[1191252](https://www.weglide.org/flight/1191252), against stations around it
reporting 1014 to 1016 all day.

Two corrections followed, and only the second one is right.

The first scaled the ISA exponent by the measured column ratio. It is wrong
because the ISA temperature is a function of *pressure* altitude, not of
height, so a ratio applied against height does not reduce the pressure at all.

The second used the relation the ratio is measured from. With `dz/dHp = τ`, a
glider at height `H` stands `H/τ` of pressure altitude above the pressure
altitude that sea level has, so the standard atmosphere read at `Hp − H/τ` is
the pressure at sea level. Where `H` is a straight line against `Hp`, that
expression is constant whatever the slope. It removed the drift, and it still
gave the wrong number, because it reduces through the real column *all the
way down*. That is a QFF. A QNH reduces through the **standard** atmosphere,
and it does so from a **field**, whose elevation therefore has to be named.

The test is on the ground, where a QNH is a definition and not an estimate.
Parked at the end of flight 1191252, the QFF form read 1015.6 hPa where the
definition gives 1021.5 hPa from the same pressure. It missed by 5.9 hPa at a
place where nothing is uncertain.

The reduction therefore splits at the field. Down to the field the column is
the real one, so the field stands `(H − E)/τ` of pressure altitude below the
glider. Below the field the standard atmosphere carries the rest. On the
ground the whole expression collapses to the definition. Measured over flight
1191252, whose field lies at 771 m, by band of pressure altitude:

| Band | ISA all the way | Real column all the way | To the field, then ISA |
| --- | --- | --- | --- |
| 600–900 m | 1022.7 | 1015.3 | 1021.4 |
| 1500–2200 m | 1032.2 | 1018.4 | 1023.4 |
| 3000–3800 m | 1042.3 | 1020.8 | 1025.3 |

The 19.6 hPa of drift falls to 3.9 hPa, and what is left is the ratio, not the
form. A fixed 1.064 flattens the flight to 0.94 hPa, and a least-squares line
of `H` against `Hp` over the whole flight gives 1.0599, which is inside that
optimum. The block estimator wanders between 1.052 and 1.071 instead, and
0.004 of ratio is worth about 1 hPa per 1000 m of climb.

#### The elevation cannot be derived from the flight

The elevation was derived from the flight itself: follow the height while the
glider stands still, and stop at the first movement. It fails in two ways.

Wave breaks the test for standing still. A glider parked in wave holds its
position over the ground for minutes, so an estimator started there reads a
ground speed of zero and takes its flight altitude for a field. Fed 3000 m at
0.4 m/s over the ground, it latched 3086 m and reported 1013.2 hPa, which is
inside the range a pilot would accept and means nothing. That is worse than
the QFF it replaced, which at least missed by a known amount in a known
direction.

Three further conditions narrow it. The height has to be steady, which rejects
wave lift. An airspeed, where an instrument reports one, has to read zero,
which rejects neutral wave. And the state has to hold for 30 s from the first
fix the estimator sees, which rejects a momentary standstill. One case still
passes: neutral wave, at a standstill, with no airspeed sensor, from the first
fix onward. Nothing in the ground velocity or the two altitudes separates that
from a parked glider.

The second failure has no defence at all. An application restarted in flight
never sees the ground, so it reports no setting for the rest of the day. On a
phone that is the ordinary case, not the exotic one.

`updraft_air` therefore does not report a QNH. Nothing inside the estimator
read it, and only the debug overlay showed it. The column ratio stays, because
the air density needs it.

The reduction returns when an elevation can be supplied instead of guessed:
from the pilot, from a waypoint file, or from a value kept across a restart. A
QNH entered on the ground is the most useful of the three, because it also
measures the barometer offset, which is larger than anything the reduction
does.

### The barometer offset is larger than anything the reduction does

Flight 1191252 flew from Puimoisson, and both ends of the recording sit on the
ground there. The `PowerFlarm` read 698.9 m of pressure altitude at a field
that its own receiver puts at 775 m and the published elevation puts at 780 m,
which is a QNH of 1022 to 1023 hPa. Stations across the southern Alps reported
1014 to 1016 hPa that day. The barometer is therefore 7 to 9 hPa high, or
about 60 to 70 m of pressure altitude.

No reduction can see that, and the two errors ran in opposite directions: the
QFF form read 1015 to 1021 hPa on this flight and looked correct against the
stations for the wrong reason. Only the ground comparison separates them.

The field is not flat, which matters for the elevation the setting reduces to.
The landing roll ran 725 m west and lost 32.2 m of GNSS height while losing
31.1 m of pressure altitude, so the slope is real and not a receiver error.
The two parking places of one flight differ by 30 m, or 3.5 hPa.

The **altitude** itself needs no fit. It is the fused height, which the vario
already computes, converted to mean sea level with the geoid. Against a 60 s
mean of the GNSS altitude, no crossover between 5 s and 300 s scores better
than any other, and none is far from the raw GNSS altitude: at one sample per
second the GNSS altitude is already as smooth as the barometer, 0.47 to 0.60 m
against 0.57 to 0.82 m. The gain over each source alone is not noise. It is
that the result needs no altimeter setting, carries no temperature error, and
is one value whether a barometer, a receiver, or both are connected.

Two consequences matter beyond the estimate itself.

**There are two different altitudes, and they are far apart.** At the top of
these flights the temperature term is 47 m on the coolest day and 251 m on the
warmest. Airspace and traffic separation run on what a pressure altimeter set
to QNH reads, because that is the shared reference. Terrain clearance and a
terrain map run on geometric height. The gap between them is a whole airspace
layer, so each reading has to say which one it is.

**The temperature term is the assumption the netto currently makes.** The sink
rate divides by the air density, which the estimate takes from the ISA at the
pressure altitude. `b` measures the deviation the ISA ignores, at about 1% of
sink rate per 3 K, without needing an OAT sensor.

[`updraft_egm96`]: ../../../libs/updraft_egm96

## An outside air temperature input is not worth it

All six recordings carry OAT, so the value can be measured rather than
guessed. It has two possible uses, and neither pays.

**Air density for the sink rate.** Replacing the ISA density with
`(p/p₀)·(T_ISA/T)` moves the mean sink rate by 0.002 to 0.013 m/s and the netto
difference against the instrument by 0.002 to 0.005 m/s. The two corrections
that a temperature makes almost cancel: warmer air lowers the equivalent
airspeed, which moves the polar reading down, and it divides the result by a
smaller density ratio, which moves it back up.

**The temperature term of the altitude model.** Building a temperature profile
from the flight's own OAT samples and integrating `T/T_ISA` over the pressure
altitude predicts the offset with only the altimeter setting left free. It is
worse than simply fitting the slope, on every flight:

| Flight | Fitted `a + b·Hp` | OAT profile | OAT at the glider |
| --- | --- | --- | --- |
| 1141558 | 5.2 m | 5.3 m | 8.5 m |
| 1153141 | 3.9 m | 4.8 m | 10.0 m |
| 1174605 | 5.5 m | 5.9 m | 8.0 m |
| 1179475 | 4.6 m | 4.8 m | 12.4 m |
| 1179605 | 4.3 m | 7.4 m | 15.5 m |
| 1188417 | 6.9 m | 7.6 m | 16.9 m |

Treating the temperature at the glider as the temperature of the whole column
below it is much worse again, which is expected: it is not the same quantity.

The sensor is also not neutral. At equal pressure altitude it reads 0.06 to
1.56 K warmer while circling than while cruising, because it sits in the sun
and gets less airflow. That error changes with what the pilot is doing, and it
is as large as the accuracy of the fitted term.

### OAT confirms the measured column, and does not improve it

`updraft_air` now measures the temperature of the column from the ratio of the
geometric height gained to the pressure altitude gained. OAT measures the same
quantity by a different route, so the two can be compared.

They agree. On flight 1141558 the ratio from OAT has a median of 1.0269, and
the ratio from height changes is 1.0209 over 265 blocks of 60 s, which is what
the estimator reports as a median of 1.026. The gap of 0.005 is about 1.4 K,
which moves the sink rate by 0.003 m/s.

The probe is the less trustworthy of the two. At equal pressure altitude its
reading depends on what the pilot is doing:

| Height band | OAT while circling minus while cruising |
| --- | --- |
| 600–800 m | −1.21 K |
| 800–1000 m | −1.08 K |
| 1200–1400 m | +0.83 K |
| 1400–1600 m | +1.18 K |
| 1600–1800 m | +1.23 K |

Part of that swing is real, because a thermal is warmer than the air beside it.
Part is the time of day, because low circling happens early and late while
cruising at the same height happens in the middle of the day. Neither is the
temperature of the column, and both follow the circling that the netto is read
in. The height ratio has no such correlation.

So OAT is a cross-check on the measured ratio, and not an input to it.

The one case for OAT is the start of a flight, before the glider has flown a
range of altitudes for the fit. Seeding the temperature term from OAT is better
for the first two minutes on four of the six flights, and worse from ten
minutes on. The fit reaches 0.1 to 6.5 m within ten minutes on its own, and the
temperature term is small while the glider is still low, so the gap it would
bridge is small and brief.

## A second sensor of the same kind adds nothing

Updraft can receive a position and a pressure altitude from the device it runs
on and from a connected instrument at the same time. Averaging the two only
pays off if the estimate is limited by sensor noise. It is not.

Adding synthetic noise to both altitude sources and re-running the estimate
changes the vertical speed by 0.01 m/s at 0.5 m of extra noise, and by 0.02 to
0.05 m/s at 1 m. The sources already carry 0.5 to 0.8 m, so halving that is
invisible. Adding up to 2 m/s per axis to the ground velocity changes the wind
by less than 0.1 m/s of vector RMS, which is inside the run-to-run scatter: the
wind is limited by gusts and by how often the glider turns, not by the
receiver.

What a second source is worth is therefore not accuracy:

- **Rate.** A device barometer at 10 to 50 Hz supports a shorter time constant
  than a 1 Hz instrument feed, and a shorter time constant is less lag.
- **Static source.** A device barometer measures cabin pressure, which moves
  with the vents and with airspeed. An instrument on the aircraft's static port
  does not. That is a systematic difference, so it calls for choosing a source,
  not for averaging.
- **Cover.** One recording loses its GNSS fix 15 times. A second receiver fills
  those gaps.

The same structure the [height filter](#both-altitudes-are-useful) already uses
fits all three: take the slow, trusted reference from one source and the fast
changes from the other, and check that they agree before combining them.

## A PowerFlarm recording, in two encodings

WeGlide flight 1191252 is an LS8-a, D-6814, recorded by a PowerFLARM Fusion.
The device wrote the same flight twice: an IGC file, and an NMEA stream of
`GPRMC`, `GPGGA` and `PGRMZ` sentences. Its headers name a u-blox 8 receiver
and a MEAS MS5607 barometer.

This recording has no airspeed and no instrument reference values, so it
measures the estimate against physics instead of against another vario.
Neither file is in the repository, and neither may be committed: the NMEA
stream carries the FLARM identifiers of other aircraft. Ask the repository
owner for them before you repeat this work.

### A sentence without a timestamp needs an alignment

`PGRMZ` carries no time, so it has to take the time of a fix near it in the
stream. The choice is not obvious, because the sentence sits between two
fixes.

Correlating the two height rates settles it. The correlation reaches its peak
at zero lag only when each `PGRMZ` takes the time of the fix *before* it:
0.909, against 0.832 for the fix after it. The wrong choice moves the
pressure altitude one second against the GNSS altitude, and the fused
altitude then misses the receiver's own altitude by 1.85 m RMS instead of
0.93 m.

`PAIRING_TOLERANCE` in [`HeightFilter`](#both-altitudes-are-useful) guards
against that one second, but it cannot see this route: the caller supplies
the wrong time, so the two altitudes look simultaneous. An adapter has to
settle the alignment for each device.

### A pressure altitude understates a climb on a warm day

A pressure altitude follows the ISA. The real atmosphere gives
`dz/dHp = T_real/T_ISA`, so a warm column makes the true climb larger than
the pressure altitude reports.

Measured over blocks of contiguous flight:

| Block length | Blocks | d(geometric) / d(pressure) |
| --- | --- | --- |
| 30 s | 736 | 1.0544 |
| 60 s | 350 | 1.0573 |
| 120 s | 162 | 1.0591 |

The IGC encoding of the same flight gives 1.0548 over 387 blocks of 60 s.
Split by height, 1500 to 2500 m gives 1.0611 and 2500 to 3800 m gives 1.0542.
The column was about 16 K above the ISA, which suits an August day in
Provence.

So a vario driven by pressure alone read 5.5% low on this day. The height
filter recovers the scale without being told: the offset ramps through a
climb, and its derivative restores the rate after about 5 s. Feeding the GNSS
altitude raised the vertical-speed RMS from 1.820 to 1.911 m/s, while the
step between samples stayed at 0.311 m/s against 0.314 m/s. The change is
therefore signal and not noise.

This is a second reason to fuse the two altitudes. The first is the 10% of
vertical-speed error that averaging removes.

### The ground cannot characterise an altitude source

In flight, over 85 straight steady segments, the GNSS altitude carried
0.288 m of noise and the pressure altitude 0.763 m. On the ground the GNSS
altitude carried 0.040 m, because the receiver holds its position while it
does not move.

That is the fourth measurement in this investigation that hides lag or a
removed signal behind a good noise figure, after the uBLOX LEA-4S, the Galaxy
S23 driver and the LG G7 public sensor.

### The receiver's geoid is coarse

The receiver reported a geoid separation between 47.30 and 47.50 m across
160 km of track. `updraft_egm96` gives 50.68 to 52.59 m over the same track.
A model that moves by 0.2 m across 160 km is a coarse table, so the altitude
this crate reports is closer than the `GPGGA` altitude of the same receiver.

### What the two encodings cost

The sensors and the flight are the same, so the differences belong to the
format.

| Quantity | IGC | NMEA |
| --- | --- | --- |
| Pressure altitude noise | 0.764 m | 0.762 m |
| Pressure altitude step | 1 m | 1 m, through integer feet |
| GNSS altitude step | 1 m, truncated | 0.1 m |
| Ground velocity | not recorded | reported |

The two pressure altitudes agree to a mean of +0.028 m. The integer feet of
`PGRMZ` cost nothing that can be measured, because the sensor noise is eight
times the step.

The IGC GNSS altitude is the ellipsoidal height truncated to a whole metre.
It matched `floor` on 95.3% of the shared samples and `round` on 50.0%, so it
carries a bias of −0.5 m.

The IGC file records no ground velocity, so the velocity has to come from
consecutive positions. That keeps the wind and loses the vertical speed:

| Velocity | Wind speed | Wind direction | Airspeed | Vertical-speed step |
| --- | --- | --- | --- | --- |
| Reported | 4.09 m/s | 244.9° | 133.8 km/h | 0.311 m/s |
| From positions | 4.06 m/s | 244.8° | 133.6 km/h | 0.683 m/s |

Feeding the IGC altitudes together with the reported velocity gives 0.311 m/s
again, so the velocity carries the whole difference. Positions logged to
0.001 arc-minutes gave 0.89 m/s of speed noise, against the 0.12 m/s that the
same receiver reported. The energy term multiplies an airspeed error by
`v/g`, which is 3.8 m of height for each m/s at 38 m/s.

### Two smaller results

The `FXA` extension reported 1 or 2 m on almost every fix, and 7 m at worst.
That is not a credible accuracy for this receiver. It does no harm, because
[`WindFilter`](#the-algorithm) scales its measurement variance as
`MEASUREMENT_NOISE · (1 + (accuracy / 15)²)`, which can raise the variance
and never lower it.

The IGC file drops to one record per 8 s at 94 places. All of them are on the
ground, where the ground speed stays below 15 m/s.

## Limits

- **Wind in straight flight.** Vector RMS against a 60 s mean of the recorded
  wind is 1.15 to 1.82 m/s while circling, and 2.40 to 4.92 m/s in glides. The
  recorded wind clearly tracks something during a glide that this estimate does
  not. What that is, is not established: no header names a magnetometer, and
  the V9's inertial platform gives heading *changes* rather than an absolute
  heading. Measuring the heading change independently would make the crosswind
  observable in a glide, but these recordings do not prove that is what the
  instrument does.
- **Airspeed calibration.** Fitting circles in velocity space with a free
  radius gives the airspeed the flight path implies. Against the recorded TAS,
  the median ratio runs from 0.993 to 1.047 across the fourteen flights, and it
  moves between flights of the same aircraft: 1.013 and 1.022 on HB-2393,
  1.023 to 1.047 on D-6897, and 1.003 to 1.032 on D-9486. So part of it is not
  a fixed installation error. A 4% airspeed error moves the wind along the
  heading by about 1.2 m/s. Estimating that scale as a third filter state diverges, because in
  straight flight it is degenerate with the along-heading wind; estimating it
  from complete circles only should work, and is the obvious next step.
- **The engine.** The estimate is unusable while an engine runs, and cannot
  tell that it is. See the headline section.
- **The recorded wind is noisy.** On flight 1141558 it deviates from its own
  60 s mean by 1.65 m/s. That noise also inflates the recorded wind *speed*
  relative to the magnitude of the mean wind vector, which accounts for much of
  the negative speed bias in the estimate.
- **Air density.** The estimate measures the temperature of the column instead
  of assuming the ISA one, from the ratio of the geometric height gained to
  the pressure altitude gained. No temperature input is needed. It is worth
  little: on flight 1141558 the ratio measures 1.026, and the sink rate moves
  by at most 0.02 m/s. The correction also changes sign at the best glide
  speed, +0.007 m/s at 95 km/h against −0.011 m/s at 160 km/h, so a whole
  flight barely moves. What it removes is a systematic error on a hot day,
  where the ratio reaches 1.057.
- **Flying mass.** No recording states it, so the estimate uses the polar's
  reference mass. A ballasted glider sinks faster than that.
- **One instrument manufacturer.** All fourteen recordings come from LXNAV
  instruments, and every one has an LX HAWK inertial platform.

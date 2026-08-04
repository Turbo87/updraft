# Vertical speed, netto and wind from recorded flight data

Investigation of "how close can Updraft get to an LXNAV vario, with only GNSS,
pressure altitude and true airspeed?". The answer is the `updraft_air` crate.

Measured against fourteen recordings from seven aircraft, six glider types and
five LXNAV instrument models. Only `testdata/weglide_1141558.igc` is in the
repository; the others are WeGlide flights 1113539, 1120273, 1131653, 1138165,
1140266, 1153141, 1168132, 1173566, 1174605, 1179475, 1179605, 1184098 and
1188417, and the numbers taken from them cannot be reproduced from this
repository alone.

**Headline: vertical speed and wind reach the accuracy of the recorded
instrument values while circling. Netto does not, and how far it misses is set
by the aircraft, not by the estimate.**

| Flight | Glider | Registration | Instrument | Vertical speed | Netto | Wind speed | Wind direction |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1141558 | JS-3-18m | D-KPWZ | LX9070 | 0.47 m/s | 1.27 m/s | 1.73 m/s | 17.0° |
| 1179475 | JS-3-18m | D-KPWZ | LX9070 | 0.51 m/s | 1.35 m/s | 1.92 m/s | 53.3° |
| 1188417 | JS-3-18m | OK-3314 | LX9070 | 0.72 m/s | 1.17 m/s | 2.51 m/s | 22.2° |
| 1153141 | ASH 26e | D-KAFE | LX9070 | 0.26 m/s | 1.18 m/s | 2.33 m/s | 44.2° |
| 1179605 | ASH 26e | D-KAFE | LX9070 | 0.29 m/s | 1.23 m/s | 2.40 m/s | 72.1° |
| 1138165 | ASH 25m | HB-2393 | LX9000 | 0.44 m/s | 0.89 m/s | 1.17 m/s | 24.3° |
| 1140266 | ASH 25m | HB-2393 | LX9000 | 0.51 m/s | 0.84 m/s | 1.15 m/s | 15.3° |
| 1168132 | ASW 27B | D-6897 | S100 | 0.92 m/s | 0.88 m/s | 1.23 m/s | 28.0° |
| 1173566 | ASW 27B | D-6897 | S100 | 0.92 m/s | 1.01 m/s | 1.43 m/s | 27.9° |
| 1184098 | ASW 27B | D-6897 | S100 | 0.85 m/s | 0.90 m/s | 2.00 m/s | 53.6° |
| 1113539 | LS 1 | D-9486 | S10 | 0.48 m/s | 0.86 m/s | 1.32 m/s | 20.9° |
| 1120273 | LS 1 | D-9486 | S10 | 0.53 m/s | 0.93 m/s | 1.57 m/s | 13.5° |
| 1131653 | LS 1 | D-9486 | S10 | 0.50 m/s | 0.89 m/s | 1.43 m/s | 57.7° |
| 1174605 | Duo Discus XLT | D-KBBQ | LX9000F | 0.52 m/s | — | — | — |

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
of each recording, moves that flight's netto from 2.23 to 0.89 m/s and its
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
| 1140266 | ASH 25m, HB-2393 | LX9000 | +0.98 m/s | +0.93 m/s | +0.67 m/s | +0.05 |
| 1138165 | ASH 25m, HB-2393 | LX9000 | +1.30 m/s | +1.38 m/s | +0.74 m/s | −0.08 |
| 1120273 | LS 1, D-9486 | S10 | +1.06 m/s | +1.18 m/s | +1.24 m/s | −0.12 |
| 1113539 | LS 1, D-9486 | S10 | +1.04 m/s | +1.17 m/s | +1.17 m/s | −0.13 |
| 1131653 | LS 1, D-9486 | S10 | +1.17 m/s | +1.38 m/s | +1.35 m/s | −0.21 |
| 1188417 | JS-3-18m, OK-3314 | LX9070 | +1.39 m/s | +1.09 m/s | +0.90 m/s | +0.30 |
| 1184098 | ASW 27B, D-6897 | S100 | +1.53 m/s | +1.10 m/s | +1.05 m/s | +0.43 |
| 1168132 | ASW 27B, D-6897 | S100 | +1.54 m/s | +0.78 m/s | +0.98 m/s | +0.76 |
| 1173566 | ASW 27B, D-6897 | S100 | +2.19 m/s | +0.91 m/s | +1.08 m/s | +1.28 |
| 1153141 | ASH 26e, D-KAFE | LX9070 | +1.01 m/s | +2.40 m/s | +0.84 m/s | −1.39 |
| 1179475 | JS-3-18m, D-KPWZ | LX9070 | +1.71 m/s | −0.59 m/s | +0.99 m/s | +2.30 |
| 1141558 | JS-3-18m, D-KPWZ | LX9070 | +1.94 m/s | −0.70 m/s | +0.83 m/s | +2.64 |
| 1179605 | ASH 26e, D-KAFE | LX9070 | −0.33 m/s | +2.84 m/s | +0.91 m/s | −3.17 |

Turns are samples with 25° to 70° of bank. A sink rate can never be negative,
and it cannot depend on which way the glider turns.

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
The netto barely changes (1.12 to 1.27 m/s against 1.16 to 1.35 m/s), because
its error is dominated by the recorded reference, not by the estimate.

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

`updraft_air` keeps the 2 s time constant at every rate. Shortening it needs
measurements from the sensor it would run on, which none of these recordings
contain.

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
  as correlation between consecutive differences.
- **The resolution.** A coarse quantisation step sets a floor under the noise.

Ten minutes of a still device answers all four. No flight is needed.

#### Galaxy S23 result

The first bench recording is
[`testdata/android_barometer_sm_s911b.csv`](../../../testdata/android_barometer_sm_s911b.csv).
It came from a Samsung Galaxy S23 (`SM-S911B`) with Android 16. Android
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
does not justify a common Android value. Two more device models must confirm
the result first.

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

The crate therefore reports the altimeter setting that makes a pressure
altimeter read the current altitude, which needs no extrapolation and is as
accurate as the altitude itself. It is what a pilot does when they set the
altimeter to a known height. On a day warmer than the ISA it grows with height:
over flight 1141558 it moved between 1027.7 and 1031.9 hPa.

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
- **Air density.** The estimate assumes the ISA temperature at the pressure
  altitude. The recordings carry OAT, which would remove that assumption at the
  cost of about 1% of sink rate per 3 K.
- **Flying mass.** No recording states it, so the estimate uses the polar's
  reference mass. A ballasted glider sinks faster than that.
- **One instrument manufacturer.** All fourteen recordings come from LXNAV
  instruments, and every one has an LX HAWK inertial platform.

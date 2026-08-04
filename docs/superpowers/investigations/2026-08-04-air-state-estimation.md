# Vertical speed, netto and wind from recorded flight data

Investigation of "how close can Updraft get to an LXNAV vario, with only GNSS,
pressure altitude and true airspeed?". The answer is the `updraft_air` crate.

Measured against six recordings from five gliders and two instrument
generations. Only `testdata/weglide_1141558.igc` is in the repository; the
others are WeGlide flights 1153141, 1174605, 1179475, 1179605 and 1188417, and
the numbers taken from them cannot be reproduced from this repository alone.

**Headline: vertical speed and wind reach the accuracy of the recorded
instrument values while circling. Netto does not, because the recorded netto
itself depends on the direction of turn.**

| Flight | Glider | Vertical speed | Netto | Wind speed | Wind direction |
| --- | --- | --- | --- | --- | --- |
| 1141558 | JS-3-18m | 0.48 m/s | 1.34 m/s | 1.72 m/s | 16.8° |
| 1179475 | JS-3-18m | 0.51 m/s | 1.35 m/s | 1.93 m/s | 53.6° |
| 1188417 | JS-3-18m | 0.71 m/s | 1.17 m/s | 2.52 m/s | 22.9° |
| 1153141 | ASH 26e | 0.29 m/s | 1.25 m/s | 2.40 m/s | 45.6° |
| 1179605 | ASH 26e | 0.31 m/s | 1.30 m/s | 2.62 m/s | 73.1° |
| 1174605 | Duo Discus XLT | 0.53 m/s | — | — | — |

All values are RMS differences against the recorded values. Flight 1174605
records no netto, and its 64 wind records are too few to score. Wind direction
scales with wind strength: the flights with a large error are the weak-wind
days, where the recorded wind itself drops below 2 m/s for hours.

The test `libs/updraft_air/tests/recorded_flight.rs` recomputes the 1141558 row
and keeps it in an Insta snapshot.

## Reading the error figures

An RMS difference is not the error a pilot sees. It squares each difference
before averaging, so a few large moments set the value while most samples sit
well below it. For the vertical speed the median difference is 0.16 to 0.35 m/s
against an RMS of 0.29 to 0.71 m/s, and 90% of samples are inside 0.44 to
1.13 m/s.

Three further measurements say what the figure is made of.

**Most of it is bandwidth, not error.** The recorded vario differs from its own
5 second average by 0.31 to 0.56 m/s, which is as large as the whole difference
against the estimate. That is the instrument's own second-to-second movement.
A recording of one sample per second cannot contain it, so no estimate built
from that recording can reproduce it.

**It averages away.** RMS of the difference between running averages of both
signals, by window:

| Flight | 1 s | 5 s | 15 s | 30 s | 60 s |
| --- | --- | --- | --- | --- | --- |
| 1141558 | 0.48 | 0.37 | 0.20 | 0.12 | 0.08 |
| 1179475 | 0.51 | 0.41 | 0.28 | 0.20 | 0.15 |
| 1188417 | 0.71 | 0.61 | 0.34 | 0.22 | 0.15 |
| 1153141 | 0.29 | 0.20 | 0.10 | 0.07 | 0.05 |
| 1179605 | 0.31 | 0.21 | 0.11 | 0.08 | 0.07 |
| 1174605 | 0.52 | 0.32 | 0.13 | 0.07 | 0.05 |

An error that averages away is a difference in filtering, not a wrong reading.
An error that stays is a bias.

**The averaged climb rate is accurate.** Over the 123 climbs in the six
recordings that hold 120 seconds or more of circling, the average climb rate
differs from the instrument's by 0.06 m/s on average, 0.08 m/s RMS, and never
by more than 0.29 m/s.

The netto looks worse: 1.17 to 1.35 m/s at one second, and still 0.41 to
0.92 m/s over 60 seconds. That is the recorded netto's turn offset (see below),
which is a bias in the reference and does not average away. Over stretches of
60 seconds or more with the wings level, where the offset is absent, the
averaged netto differs by 0.13 to 0.18 m/s.

## What the recordings contain

The `I` record defines FXA, SIU, ENL, TAS, GSP, TRT, VAT, OAT and ACZ
everywhere, plus NET, AOR, AOP and AOA on the LX9070 recordings. The `J` record
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

| Flight | Glider | Right turns | Left turns | Wings level |
| --- | --- | --- | --- | --- |
| 1141558 | JS-3-18m, D-KPWZ | +1.94 m/s | −0.70 m/s | +0.83 m/s |
| 1179475 | JS-3-18m, D-KPWZ | +1.71 m/s | −0.59 m/s | +0.99 m/s |
| 1188417 | JS-3-18m, OK-3314 | +1.39 m/s | +1.09 m/s | +0.90 m/s |
| 1179605 | ASH 26e, D-KAFE | −0.33 m/s | +2.84 m/s | +0.91 m/s |
| 1153141 | ASH 26e, D-KAFE | +1.01 m/s | +2.40 m/s | +0.84 m/s |

Turns are samples with 25° to 70° of bank. A sink rate can never be negative,
and it cannot depend on which way the glider turns.

The asymmetry follows the **aircraft**, not the firmware. Both D-KPWZ flights
read high to the right, both D-KAFE flights read high to the left, and OK-3314
is nearly symmetric even though all three fly the same LX9070 firmware
generation. That points at the pressure-port installation or a steady sideslip,
not at the netto formula.

Wings level, where the artifact is absent, the recorded sink rate lands between
0.83 and 0.99 m/s on every flight, against a modelled 0.89 m/s. On flight
1141558, speed bins of 10 km/h agree within ±0.22 m/s with no trend against
speed. The netto model is therefore sound, and the 1.2 to 1.4 m/s RMS in the
first table measures the instrument more than the estimate.

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
  the median ratio is 0.993 to 1.002 on three aircraft but 1.033 to 1.040 on
  the other two. A 4% airspeed error moves the wind along the heading by about
  1.2 m/s, and the two aircraft with the offset are the two with the worst wind
  results. Estimating that scale as a third filter state diverges, because in
  straight flight it is degenerate with the along-heading wind; estimating it
  from complete circles only should work, and is the obvious next step.
- **The recorded wind is noisy.** On flight 1141558 it deviates from its own
  60 s mean by 1.65 m/s. That noise also inflates the recorded wind *speed*
  relative to the magnitude of the mean wind vector, which accounts for much of
  the negative speed bias in the estimate.
- **Air density.** The estimate assumes the ISA temperature at the pressure
  altitude. The recordings carry OAT, which would remove that assumption at the
  cost of about 1% of sink rate per 3 K.
- **Flying mass.** No recording states it, so the estimate uses the polar's
  reference mass. A ballasted glider sinks faster than that.
- **One instrument manufacturer.** All six recordings come from LXNAV
  instruments, and five from the same firmware generation.

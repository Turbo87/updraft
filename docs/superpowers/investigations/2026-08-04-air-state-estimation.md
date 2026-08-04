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

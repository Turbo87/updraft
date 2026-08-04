# Vertical speed, netto and wind from recorded flight data

Investigation of "how close can Updraft get to an LXNAV vario, with only GNSS,
pressure altitude and true airspeed?". The answer is the `updraft_air` crate.
Measured against `testdata/weglide_1141558.igc`, a five-hour cross-country
flight in a JS-3-18m logged by an LX9070 with a V9 vario.

**Headline: vertical speed and wind reach the accuracy of the recorded
instrument values. Netto does not, because the recorded netto itself is wrong
in circling flight.**

| Quantity | n | RMS | MAE | Bias | Correlation |
| --- | --- | --- | --- | --- | --- |
| Vertical speed | 18811 | 0.53 m/s | 0.40 m/s | −0.01 m/s | 0.96 |
| Netto | 18811 | 1.35 m/s | 1.01 m/s | −0.14 m/s | 0.72 |
| Wind speed | 13357 | 1.72 m/s | 1.31 m/s | −0.25 m/s | 0.53 |
| Wind direction | 13357 | 16.8° | 12.6° | −1.1° | — |

The test `libs/updraft_air/tests/recorded_flight.rs` recomputes this table and
keeps it in an Insta snapshot.

## What the recording contains

The `I` record defines FXA, SIU, ENL, TAS, GSP, TRT, VAT, OAT, NET, ACZ, AOR,
AOP and AOA. The `J` record defines WDI and WSP. LXNAV writes speeds as
hundredths of a kilometre per hour, vertical speeds as hundredths of a metre
per second, accelerations as hundredths of *g*, and roll and pitch in whole
degrees.

Two independent checks confirm the field offsets. ACZ reads 1.00 in level
flight, and ACZ matches `1/cos(AOR)` across the flight.

The estimate uses TRT, GSP, FXA, the pressure altitude and TAS. It uses neither
the position nor the GNSS altitude. Track and ground speed already carry the
ground velocity, and the pressure altitude replaces the GNSS altitude, which is
too noisy to differentiate. Deriving the ground velocity from consecutive
positions instead of TRT and GSP gives the same wind accuracy, so a recording
without those extensions loses nothing.

## The algorithm

**Vertical speed.** The total energy height `h + v²/2g` removes the height that
the glider trades against airspeed. Its derivative is the total-energy vertical
speed. Two exponential stages with a 2 s time constant smooth it.

The compensation is what makes this work. Against the recorded vario, the
unsmoothed pressure-altitude rate correlates 0.73. The two-stage filter on the
total energy height correlates 0.96. An optimal linear filter, fitted on the
whole flight, reaches 0.965. The simple two-stage filter is therefore already
at the limit of what these inputs support.

**Wind.** Each sample states that `TAS = ‖ground velocity − wind‖`. That is one
scalar measurement of a two-component state, so an extended Kalman filter
tracks the wind vector. The measurement constrains the wind along the current
heading only. A turn is what makes the wind observable, and one full circle is
enough to converge.

This one filter replaces the usual split between a circling method and a
straight-flight method. In a circle the heading sweeps every direction, so the
filter behaves like a circle fit in velocity space. In a glide it still
corrects the along-heading component, which is why the estimate does not go
stale between thermals. A circle fit that holds its last value between circles
scores 3.14 m/s vector RMS against the recorded wind; the filter scores
2.77 m/s.

The GNSS fix accuracy scales the measurement variance. The wind is reported
only after the filter converges, which took 118 s in this recording.

**Netto.** Subtracting the wind from the ground velocity gives the air-relative
heading. Its rate of change is the turn rate, which fixes the bank angle and
the load factor `n = √(1 + (ω·v/g)²)`. The derived bank angle correlates 0.982
with the recorded AOR, which confirms the wind estimate independently.

A glide polar is quoted as equivalent airspeed against sink rate at sea level.
Both axes scale with `1/√σ` at a density ratio `σ`, and both scale with `√n` in
a turn, so the polar is read at `v·√σ/√n` and its result is scaled back. The
netto is the vertical speed plus that sink rate.

## The recorded netto is not a usable reference

The recorded netto minus the recorded vario is the sink rate that the
instrument applied. It depends on the direction of turn:

| Flight state | n | Recorded sink | Modelled sink |
| --- | --- | --- | --- |
| Right turns, 25°–70° of bank | 4458 | +1.94 m/s | ≈0.66 m/s |
| Left turns, 25°–70° of bank | 1794 | −0.70 m/s | ≈0.66 m/s |
| Wings level | 10215 | +0.83 m/s | 0.89 m/s |

A sink rate can never be negative, and it cannot depend on which way the glider
turns. The instrument's angle of attack also differs between the two turn
directions, so the asymmetry is in its air data or its inertial platform, not
in the netto formula alone. The mean of the two turn directions, 0.62 m/s,
matches the modelled 0.66 m/s.

In wings-level flight, where the artifact is absent, the recorded sink rate
averages 0.06 m/s below the model. Speed bins of 10 km/h agree within
±0.22 m/s, with no trend against speed. Those bins still mix in pull-ups and
push-overs, where the instrument's netto and vario have different lags, so part
of that spread is not a polar error. The netto model is therefore sound, and
the 1.35 m/s RMS in the table measures the instrument's error more than the
estimate's.

## Limits

- **Wind in straight flight.** Vector RMS is 1.03 m/s while circling and
  2.66 m/s in glides, both against a 60 s mean of the recorded wind. The
  instrument has a magnetic heading and can measure the crosswind component
  directly. Without a heading, a glider must turn.
- **The recorded wind is noisy.** It deviates from its own 60 s mean by
  1.65 m/s. Much of the 1.72 m/s RMS in the table is that noise.
- **Air density.** The estimate assumes the ISA temperature at the pressure
  altitude. The recording carries OAT, which would remove that assumption at
  the cost of about 1% of sink rate per 3 K.
- **Flying mass.** The recording does not state it, so the estimate uses the
  polar's reference mass. A ballasted glider sinks faster than that.
- **One recording.** Every number here comes from a single flight, a single
  glider and a single instrument.

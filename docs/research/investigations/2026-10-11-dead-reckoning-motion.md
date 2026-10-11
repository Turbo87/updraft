# Motion models for dead reckoning

This investigation compares motion models for the dead reckoning of ownship
and traffic symbols. Dead reckoning moves a symbol from its latest received
position until the next position arrives. The models predict 0.5 to 1.5
seconds after a fix. The
[dead-reckoning spec](https://github.com/Turbo87/updraft/issues/640) asks for
this comparison before the core publishes motion values.

The main results are:

- For ownship, an arc with the turn rate from the latest two RMC tracks is the
  best model. Its RMS error at 1.0 s is 0.49 m. A straight line gives 2.18 m.
- Smoothing of the ownship turn rate makes the result worse.
- A turn rate limit of 20 °/s or more has no measurable effect on ownship. A
  limit of 30 °/s is above all observed turn rates.
- For traffic, a straight line with the reported FLARM speed and track is the
  best model overall. Turn rates from accepted positions, from reported
  tracks, or from both do not improve it.
- The circle fit of the spec helps only a little in steady turns. It is much
  worse in straight flight.

The traffic result does not agree with the traffic motion of the spec. The
spec fits a circle to the accepted positions to get the track and the turn
rate.

## Data and method

The corpus has one long NMEA recording with FLARM traffic from one
PowerFLARM device. It has 23,379 ownship fixes and 110,489 accepted traffic
positions of 290 targets. The recording is retained privately because it can
contain personal data. Same-day IGC files from several traffic targets supply
an independent reference.

The bench is `libs/updraft_core/examples/dead_reckoning_bench.rs`. Run it on a
directory of NMEA and IGC files:

```sh
cargo run -p updraft_core --example dead_reckoning_bench -- <directory>
```

Ownship uses the RMC fixes. Traffic uses the positions that the core accepts
with the FLARM position correction. Each accepted position has the GPS time of
its reference fix. The reported speed and track come from the PFLAA report
that supplied the position.

Each model sees the positions of the last 5 seconds, at most 5 positions. It
predicts the position at 0.5, 1.0, and 1.5 seconds after the fix time. The
bench scores the prediction against three references:

- Ownship against later RMC fixes.
- Traffic against later accepted positions.
- Traffic movement against the IGC file of the target.

A Catmull-Rom spline through the later fixes supplies positions between fixes.
The spline needs four fixes at one-second intervals. The IGC comparison uses
movement, not position. It compares the predicted movement from the received
position with the IGC movement from the fix time. The received FLARM position
has an RMS offset of 34.69 m from the IGC position. This offset does not
depend on the motion model, so the comparison removes it.

The bench matches a target to an IGC file when the median distance between
the positions is less than 50 m over at least 120 shared fixes. A target
matches at most one IGC file. Only IGC files with one-second fixes can match.
Nine targets matched.

The bench scores only fixes with a ground speed of at least 15 m/s. It labels
each fix with a flight phase from the path between 1 second before and 3
seconds after the fix. A fix is straight below
4 °/s and turning from 8 °/s. A change of at least 6 °/s within two seconds
labels a turn entry or exit.

## Compared models

| Model | Track | Turn rate |
| --- | --- | --- |
| Hold | None | None |
| Straight | Reported | Zero |
| Straight, published track | Published | Zero |
| Track rate | Reported | Latest change of the reported track |
| Track rate, 1 s and 2 s average | Reported | Exponential average of the track changes |
| Step turn rate | Reported | Reported track against the bearing of the latest step |
| 3-point circle | Circle tangent | Circle through the latest 3 positions |
| Fitted circle | Circle tangent | Least-squares circle through the window |
| Circle turn rate | Reported | Least-squares circle through the window |
| Quadratic | None | Quadratic least-squares fit of the positions over time |

All models except Quadratic move along a circular arc with the reported
speed. The published traffic track is the bearing between the latest two
accepted positions. For ownship, the reported and published tracks are the
same RMC track. Quadratic replaces the spline extrapolation baseline of the
spec. A spline also extrapolates with the polynomial of its last segment.

Step turn rate uses only the latest 2 positions and the reported track. On a
circle, the bearing of a step lags the track at its end by half the turn of
the step. Twice the difference, divided by the step time, gives the turn
rate.

## Ownship

RMS error in meters against later fixes:

| Model | All, 1.0 s | All, 1.5 s | Straight, 1.0 s | Turning, 1.0 s | Turning, 1.5 s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hold | 38.08 | 57.08 | 40.23 | 31.26 | 46.67 |
| Straight | 2.18 | 4.89 | 0.38 | 4.52 | 10.15 |
| Track rate | 0.49 | 1.12 | 0.28 | 0.73 | 1.59 |
| Track rate, 1 s average | 0.55 | 1.25 | 0.31 | 0.79 | 1.73 |
| Track rate, 2 s average | 0.61 | 1.38 | 0.35 | 0.86 | 1.90 |
| Step turn rate | 0.49 | 1.11 | 0.28 | 0.72 | 1.57 |
| 3-point circle | 0.73 | 1.55 | 0.39 | 1.08 | 2.21 |
| Fitted circle | 1.93 | 3.42 | 1.59 | 2.32 | 4.22 |
| Circle turn rate | 0.81 | 1.83 | 0.51 | 1.12 | 2.48 |
| Quadratic | 3.65 | 6.65 | 0.92 | 7.38 | 13.49 |

The sample has 16,649 straight fixes and 4,839 turning fixes at 1.0 s.

The RMC track comes from the receiver velocity. It is more precise than the
direction between positions. Models that use the RMC track are better than
models that use only positions. Position errors of a GPS receiver change
slowly. A circle through 5 positions over 4 seconds therefore follows a wrong
curvature for some time. The fitted circle is worse than the 3-point circle,
also in straight flight.

The latest track change is the best turn-rate source. An average follows
changes of the turn rate later. This is most visible at a turn entry or exit.
There, the 1.0 s RMS error is 1.36 m without an average and 1.55 m with a
1-second average. Step turn rate gives the same result as the track rate.
It does not need a second track.

While turning, the turn rate of the track-rate model has a median of 16.3 °/s.
Its 99th percentile is 24.0 °/s, and its maximum is 28.7 °/s. Limits from
20 °/s to 40 °/s do not change the 1.5 s RMS error. A limit of 15 °/s raises
the turning error from 1.59 m to 2.25 m.

## Traffic

RMS error in meters against later accepted positions:

| Model | All, 1.0 s | All, 1.5 s | Straight, 1.0 s | Turning, 1.0 s | Turning, 1.5 s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hold | 43.31 | 64.50 | 44.37 | 42.88 | 63.72 |
| Straight | 10.22 | 12.71 | 3.81 | 15.64 | 20.28 |
| Straight, published track | 10.22 | 14.69 | 3.64 | 14.95 | 23.35 |
| Track rate | 11.89 | 16.69 | 4.02 | 19.45 | 29.03 |
| Step turn rate | 18.37 | 30.75 | 5.04 | 32.88 | 57.16 |
| 3-point circle | 10.95 | 16.15 | 4.96 | 13.17 | 19.53 |
| Fitted circle | 14.00 | 21.01 | 11.30 | 13.15 | 18.92 |
| Circle turn rate | 11.68 | 16.45 | 4.70 | 18.33 | 26.46 |
| Quadratic | 15.38 | 23.64 | 11.30 | 19.02 | 29.78 |

RMS error in meters of the movement against IGC files:

| Model | All, 1.0 s | All, 1.5 s | Straight, 1.0 s | Turning, 1.0 s | Turning, 1.5 s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hold | 39.94 | 59.78 | 44.32 | 34.29 | 51.27 |
| Straight | 8.91 | 12.75 | 4.10 | 14.00 | 20.59 |
| Straight, published track | 8.93 | 14.19 | 4.37 | 14.00 | 23.25 |
| Track rate | 10.76 | 16.70 | 4.31 | 17.63 | 28.04 |
| Step turn rate | 15.13 | 25.42 | 5.47 | 25.13 | 42.18 |
| 3-point circle | 10.07 | 15.93 | 6.24 | 14.36 | 23.25 |
| Fitted circle | 11.75 | 18.49 | 10.94 | 12.92 | 20.52 |
| Circle turn rate | 9.95 | 15.09 | 4.81 | 15.46 | 23.45 |
| Quadratic | 17.76 | 28.99 | 8.77 | 29.95 | 49.43 |

The IGC comparison has 6,107 straight fixes and 2,381 turning fixes at 1.0 s.
Both references select the straight model.

The reported track is better than the published track at 1.5 s. The published
track is the direction of the last step, so it lags in a turn. The reported
track applies to the accepted position well enough for a straight line.

FLARM positions have meter resolution and position jitter. A turn rate from 3
to 5 such positions is noisy. In straight flight, the noise makes the fitted
circle 2.7 times worse than a straight line against IGC. In turns, the fitted
circle reduces the 1.0 s error from 14.00 m to 12.92 m. At 1.5 s, the gain is
0.07 m. A turn rate limit does not remove the straight-flight noise. The best
limit of the circle turn rate model, 15 °/s, gives 14.61 m at 1.5 s. The
straight model gives 12.75 m.

The turn rate from consecutive reported tracks is also worse than a straight
line. Its 99th percentile while turning is 89 °/s against IGC. The change of
the reported track from one report to the next is too noisy for a turn rate.

Step turn rate is the worst traffic model in turns. While turning, its median
turn rate is 57.2 °/s against later accepted positions. A real turn rate is
less than half of this value. The reported track is therefore ahead of the
step bearing by more than half the turn of a step. 2 positions and the
reported track do not supply a turn rate.

## Slow aircraft

The corpus has only 42 ownship fixes below 15 m/s. Below 2 m/s, the straight
model has a 1.0 s RMS error of 0.27 m, and Hold has 0.94 m. FLARM reports a ground speed of
zero on the ground, so slow traffic does not move in any model. The data does
not contradict a dead-reckoning limit of about 2 m/s. It also does not select
an exact value.

## Excluded data

The committed fixture `testdata/flight_1.nmea` comes from a different FLARM
device. Its sentence order does not match the FLARM position correction. Its
accepted traffic positions alternate between short and long steps. Its traffic
errors do not describe the models. The bench test keeps the fixture only as a
regression check of the bench output.

## Limits

All results come from one FLARM device, one GPS receiver, and one day. The
ownship turns are those of one glider.

The bench predicts from the fix time. Dead reckoning starts at the receive
time. The delay between the two is unknown. The bench does not measure the
effect of this delay.

The traffic phase labels use the noisy FLARM positions for the comparison with
accepted positions. Many fixes therefore count as a turn entry or exit. The
IGC labels are less affected.

The movement comparison with IGC files removes a constant offset. Part of the
34.69 m offset can be a time difference between the FLARM and IGC positions.
In a turn, a time difference also rotates the IGC movement. This can increase
the turning errors against IGC for all models.

The spline reference adds an error between fixes. For a one-second spacing,
this error is small in comparison with the traffic errors. It can contribute
to the ownship errors at 0.5 s and 1.5 s.

## Implications for Updraft

- For ownship motion, use the GPS ground speed and track. Calculate the turn
  rate from the latest track change over the GPS fix interval, without
  smoothing. The window is the latest 2 fixes. Use a limit of 30 °/s, which
  is above the observed maximum of 28.7 °/s.
- For traffic motion, use the reported FLARM ground speed and track, without
  a turn rate. Traffic motion then needs no position window and no circle
  fit. This changes the traffic motion of the spec.
- These results do not select the minimum speed of about 2 m/s. The spec of
  the ownship-motion slice must set it.

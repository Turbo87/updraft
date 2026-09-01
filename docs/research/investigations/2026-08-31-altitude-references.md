# Altitude references and atmospheric inputs

This investigation asks which altitude sources and atmospheric inputs improve
Updraft's sensor fusion.

The accepted design is small:

- Pressure altitude supplies fast changes.
- GNSS altitude supplies the slow reference.
- Updraft converts the combined ellipsoid altitude to mean sea level with the
  EGM96 geoid model.
- Netto uses standard-atmosphere density at the raw pressure altitude.

Experiments with delayed-GNSS detection, an in-flight QNH estimate, outside air
temperature, and simple sensor averaging did not justify more estimator state.

## Why the two altitudes differ

Pressure altitude uses the standard pressure datum of 1013.25 hPa. A GNSS
receiver measures geometric altitude against an ellipsoid. The two values can
differ by more than 100 m because of:

- the pressure setting;
- the difference between the ellipsoid and mean sea level;
- temperature differences from the standard atmosphere;
- sensor and installation errors.

The values cannot be averaged directly. Their short-term changes can still be
combined. Pressure altitude normally arrives faster and responds quickly. GNSS
altitude gives a stable long-term reference that does not depend on local
pressure.

The result serves geometric uses such as terrain clearance. It is not the same
as a pressure altimeter set to QNH. Airspace and traffic separation can require
the pressure reference instead. A user interface must name the reference when
it presents either value.

## The five-second altitude filter

The filter tracks the difference between GNSS and pressure altitude. It applies
that difference as a slowly changing offset to pressure altitude. The offset
uses a five-second time constant.

This gives pressure altitude the fast changes and GNSS altitude the slow
changes. The first matched pair can change the reference by a large amount. The
vario removes that reference step before it calculates vertical speed.

On five recordings with correctly aligned GNSS data, combining the sources
improved or preserved every total-energy vertical-speed result:

| Flight | Pressure only | Both altitudes |
| --- | ---: | ---: |
| [1141558](https://www.weglide.org/flight/1141558) | 0.52 m/s | 0.47 m/s |
| [1153141](https://www.weglide.org/flight/1153141) | 0.26 m/s | 0.25 m/s |
| [1179475](https://www.weglide.org/flight/1179475) | 0.59 m/s | 0.51 m/s |
| [1179605](https://www.weglide.org/flight/1179605) | 0.30 m/s | 0.29 m/s |
| [1188417](https://www.weglide.org/flight/1188417) | 0.87 m/s | 0.72 m/s |

The mean RMS difference fell from 0.51 to 0.45 m/s. This is an improvement of
about 12%.

## Delayed GNSS altitude

One older receiver logged GNSS altitude about one second late. Its altitude
rate correlated 0.927 with the current pressure-altitude rate and 0.993 with the
previous sample. Combining it as if both values were current made the
vertical-speed result 32% worse.

An experiment compared recent GNSS and pressure-altitude rates. It stopped
using GNSS when the previous pressure sample fit better. The detector handled
the observed recording, but it was not kept.

The detector added history and thresholds for one failure shape. It could not
correct a caller that assigned a plausible but wrong timestamp. It could also
reject useful GNSS corrections during real changes.

The accepted boundary is earlier in the data path. A device adapter must assign
the time that the device measurement represents. The altitude filter only
pairs samples within 200 ms of their supplied timestamps. If a source cannot
provide reliable timing, the adapter should not present its altitude as a
current value.

## Why sensor fusion does not estimate QNH

QNH is a pressure setting for a named field elevation. It reduces measured
pressure to sea level through the standard atmosphere below that field. A
glider in flight does not supply the field elevation that the definition needs.

An experiment fitted the difference between GNSS and pressure altitude. It
could separate a pressure offset from the temperature of the air column only
after the aircraft covered a useful height range. Near the ground, many splits
fit equally well. After an application restart in flight, no ground reference
exists at all.

Wave flight creates another false ground case. An aircraft can remain nearly
stationary over the ground while it is thousands of metres high. Extra checks
for steady altitude, zero airspeed, and a 30-second wait reduce this risk but do
not remove it when no airspeed sensor is present.

The estimator therefore does not report QNH. A later feature can accept a known
field elevation, a pilot-entered QNH, or a value retained across a restart.
Pilot input on the ground is strongest because it also measures barometer
offset.

## The PowerFlarm pressure offset

[WeGlide flight 1191252](https://www.weglide.org/flight/1191252) provides a
useful check. The PowerFlarm recorded about 699 m of pressure altitude while
parked at a field near 780 m. This implies a QNH of about 1022 to 1023 hPa.
Nearby weather stations reported about 1014 to 1016 hPa.

The 7 to 9 hPa difference is too large for an in-flight temperature correction
to explain. It points to barometer or installation offset. A calculation that
reduced the real air column to sea level happened to produce values near the
weather stations, but it did so for the wrong reason. The ground comparison
exposed the error.

The landing roll also crossed sloping ground. The two parking positions differed
by about 30 m. A field elevation must therefore name the point to which the
pressure setting refers.

This result supports explicit ground calibration. It does not support deriving
QNH from the flight alone.

## Outside air temperature

The recordings include outside air temperature. Two possible uses were tested.

The first use corrected air density for the glider polar. It changed the mean
sink rate by 0.002 to 0.013 m/s. The netto difference moved by only 0.002 to
0.005 m/s. Warmer air changes both the equivalent airspeed and the conversion
back to true sink rate. The two effects almost cancel.

The second use built a temperature profile for the altitude difference. It was
worse than fitting the relationship between GNSS and pressure altitude on all
six tested recordings. Using only the temperature at the aircraft was worse
again because it does not describe the complete air column below.

The probe also changed with flight condition. At equal pressure altitude, its
reading differed by as much as 1.56 K between circling and cruising. Sun and
airflow can affect the sensor.

Outside air temperature is useful supporting evidence. Its measured benefit is
too small and uncertain to add it to the current estimator.

## More sensors do not mean more accuracy

Adding a second source helps only when random sensor noise limits the result.
The recorded estimator is mainly limited by response, gusts, source timing, and
systematic installation effects.

Adding 0.5 m of synthetic noise to both altitude sources changed vertical speed
by about 0.01 m/s. Adding 1 m changed it by 0.02 to 0.05 m/s. Adding up to
2 m/s of noise to each ground-velocity axis changed wind by less than 0.1 m/s
RMS.

A second source still has three practical uses:

- A faster barometer can reduce response delay after its bandwidth is checked.
- An aircraft static port can avoid cabin-pressure changes from vents and
  airspeed.
- A second receiver can cover a loss of GNSS fixes.

These uses require source selection and continuity handling. Simple averaging
would hide systematic differences and source changes.

## Device timestamps belong in the adapter

The PowerFlarm also wrote an NMEA stream. Its `PGRMZ` pressure-altitude sentence
has no timestamp and sits between two position fixes.

The altitude-rate correlation identifies the correct pairing. Assigning each
`PGRMZ` sentence to the previous fix gave a correlation of 0.909. Assigning it
to the next fix gave 0.832. The wrong choice moved pressure altitude by one
second and increased the combined-altitude RMS from 0.93 to 1.85 m.

The altitude filter cannot discover this error. It sees only the timestamp that
the caller supplies. The NMEA adapter must define the timing rule for the
device before it sends values to sensor fusion.

This is why timing is part of ingestion rather than a repair inside the
estimator. The same rule applies to delayed receiver output and to different
clocks on Android sensors.

## Limits

- Most source recordings are retained privately because they contain personal
  or traffic identifiers.
- GNSS altitude can contain receiver filtering and delay that are not visible
  in a simple noise figure.
- QNH requires an external field reference or pilot input.
- The current filter assumes that adapters supply meaningful timestamps.
- The density and temperature tests cover one instrument family.
- The geometric altitude is suitable for terrain-related use. It does not
  replace a shared pressure altitude for airspace or traffic separation.

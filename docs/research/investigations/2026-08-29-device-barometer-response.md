# Device barometers and vario response

This investigation asks whether a phone barometer can use less smoothing than
the fixed two-second vario setting.

Two bench recordings show that some device barometers are quiet enough for a
faster response. They also show why Updraft must not choose the response from
noise alone. A sensor driver can remove both noise and useful movement before
the application receives a sample. The result looks quiet but arrives late.

An adaptive smoothing experiment was implemented and tested. It was not kept.
Updraft continues to use a fixed two-second time constant until a source can be
checked for noise, bandwidth, and delay together.

## What a shorter time constant changes

The vario uses two smoothing stages. Each stage currently has a two-second time
constant. Together, they add about four seconds of response delay.

A shorter time constant reduces that delay. It also lets more altitude noise
reach the vertical-speed result. A useful source must therefore satisfy two
conditions:

- Its output must be quiet enough at the shorter setting.
- Its output must still contain the fast pressure changes that the vario needs.

Sample rate does not answer either question by itself. A driver can repeat or
smooth measurements at a high rate. A bench recording can measure noise and
show signs of filtering. A flight recording with an independent reference is
needed to measure the complete response.

The one-second IGC recordings used for the main
[sensor-fusion investigation](2026-08-04-sensor-fusion.md) cannot test a vario
response much faster than two seconds.

## Galaxy S23 bench result

A Samsung Galaxy S23 supplied 15,002 pressure samples in 600.04 seconds. The
mean rate was 25.000 Hz. The median gap was 40.000 ms, and no gap exceeded
41 ms.

The measured pressure noise was equal to 0.0240 m of altitude. Replaying the
recording through the two vario filters gave these results:

| Time constant | Full recording RMS | Highest one-minute RMS | Approximate delay |
| --- | ---: | ---: | ---: |
| 0.200 s | 0.104 m/s | 0.121 m/s | 0.400 s |
| 0.225 s | 0.092 m/s | 0.110 m/s | 0.450 s |
| 0.250 s | 0.083 m/s | 0.101 m/s | 0.500 s |

A 0.25-second time constant stayed below the experimental noise limit of
0.12 m/s. This result shows that one device can support a faster filter. It
does not establish a safe value for Android devices in general.

The samples were not independent. The phone or its driver had already
smoothed the pressure stream. A stationary recording has no known pressure
step, so it cannot measure the resulting delay directly.

## LG G7 bench result

An LG G7 exposed two pressure streams at the same time. Its public Android
sensor delivered 25 Hz. An LG-specific unfiltered sensor delivered 32 Hz.

Both streams were quiet enough for a 0.25-second vario time constant:

| Source | Rate | Altitude noise | Vario RMS at 0.25 s |
| --- | ---: | ---: | ---: |
| Public | 25 Hz | 0.000476 m | 0.0119 m/s |
| Unfiltered | 32 Hz | 0.003116 m | 0.0415 m/s |

The public stream looked better if only noise was considered. The simultaneous
unfiltered stream showed the missing part of the result. At 0.1 Hz, the public
stream was about 3 dB lower and about 1.4 seconds late. It had removed useful
pressure movement before Android delivered the data.

The unfiltered stream had more noise but much less delay. Its private sensor
type is specific to LG, so a portable Android adapter cannot depend on it.

This comparison gives the decisive result: lower measured noise can mean more
driver filtering, not a better vario source.

## The adaptive experiment

The adaptive implementation estimated altitude noise from recent pressure
samples. It connected two calibration points with a curve:

- About 0.6 m of noise used the two-second time constant.
- About 0.024 m of noise used a 0.25-second time constant.

It limited the result to 0.25 through 3 seconds. Slow or coarsely rounded
sources kept at least the two-second default.

The experiment had three problems. First, two devices do not define a general
sensor model. Second, noise does not measure driver delay. Third, the estimator
measured pressure noise but calculated vertical speed from pressure and GNSS
altitude together. GNSS corrections can also affect the combined altitude.

The repository's one-second flight fixture did not exercise the adaptive path.
Its sample gaps exceeded the experiment's 200 ms limit, so every sample used
the fixed fallback. An unchanged flight snapshot only tested that fallback.

The implementation was removed from the accepted branch. The fixed two-second
time constant remains in production code.

## Evidence needed to revisit the decision

A future test must record a device and an aircraft instrument during the same
flight. It must contain separate rows and timestamps for:

- raw pressure from the device;
- GNSS altitude, track, ground speed, and accuracy;
- true airspeed when an instrument supplies it;
- the instrument's pressure altitude and vario.

The clocks must have a known mapping. An IGC file cannot store the required
multi-rate streams. A fused one-second row also removes the information that
the test needs.

The flight should last at least two hours. It should include launch, several
climbs, a fast glide, and changes to vents or canopy airflow. Bench recordings
from several device models should measure quiet operation first.

The decision can be revisited when the shorter response improves delay without
adding unreadable noise or following GNSS correction steps. Until then, a
caller-supplied or automatically selected shorter setting would promise more
than the evidence supports.

## Limits

- The bench recordings are retained privately and are not repository fixtures.
- The S23 recording has no independent pressure step, so its driver delay is
  not known.
- The LG result measures one old device and one manufacturer-specific sensor.
- A stationary device does not show cabin-pressure changes caused by airspeed,
  vents, or canopy airflow.
- The results reject automatic selection from noise alone. They do not show
  that every device barometer needs two seconds of smoothing.

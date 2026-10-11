# Flight Data

Status: Current behavior

The core selects current GPS, pressure-altitude, and true-airspeed values from
typed source candidates. Each domain selects a source independently.

## Sources and priority

External devices use their saved order as priority. The first enabled device
with a fresh candidate for a domain wins.

Internal GNSS is the final fallback for GPS. It does not provide pressure
altitude or true airspeed. A device can supply more than one domain, and
different domains can select different devices.

A disconnected source keeps its configured priority. Selection depends on the
age of its latest accepted values, not its connection-state label.

## Freshness

Each accepted field stores the shell-supplied monotonic ingestion time. A value
is fresh while its age is less than three seconds. It is stale at three seconds
or later.

The core evaluates freshness when it receives data, a configuration change, or
a tick. A fresh higher-priority candidate replaces the current source
immediately.

When no candidate is fresh, the selected domain becomes last known. The
published value remains present with `stale: true`. A new fresh candidate makes
the domain current again.

Disabling, editing, or deleting the selected source clears that source's
runtime candidates. The core selects a fresh fallback. If no fallback exists,
the affected selected value becomes unavailable instead of retaining data from
the explicitly reset source.

Reordering does not clear candidates. It selects again with the new priority.

## GPS

GPS requires a fresh position. Optional altitude, track, ground speed, and fix
time come from the same source and remain present only while each field is
fresh.

Accepted RMC messages can update position, track, ground speed, and UTC time.
The core ignores an inactive RMC status and the explicit not-valid positioning
mode. Accepted GGA messages can update position, MSL altitude, and UTC time of
day. The core ignores an invalid GGA fix quality.

An external source can combine RMC and GGA fields. The position ingestion time
anchors the selected GPS snapshot.

Each GPS source calculates a ground-track turn rate. The turn rate is the
signed change from the previous track to the new track, divided by the GPS fix
interval. A right turn is positive. The core does not smooth the turn rate, and
it does not limit it. The turn rate needs two tracks with fix times at most
3 seconds apart. A repeated fix time keeps the previous turn rate. A fix
without a time, an earlier fix time, or a longer interval clears it. The turn
rate stays fresh only as long as its track.

This turn rate supplies dead reckoning of the ownship symbol. It is separate
from the turn rate that the estimator uses for the bank angle.

Android GNSS supplies a complete UTC instant and ellipsoid altitude. The core
uses EGM96 to convert ellipsoid altitude to mean-sea-level altitude.

The core calculates the solar position for the selected ownship position. It
prefers a complete UTC instant from the selected GPS source. It uses the device
UTC instant when the GPS source has no date. Monotonic elapsed time advances
both UTC sources between reports.

## Position jumps

A position jump is a change of position of the selected GPS that no glider can
fly. A fix is a candidate when it is more than 2 km from the last accepted fix
and the implied ground speed is more than 150 m/s. The implied speed uses the
UTC that the flight recording uses, so a replay with speed-up gives no jumps.
The core ignores a fix with the same UTC. A fix with an earlier UTC, or a UTC
more than 3 hours later, restarts the check from that fix. After a restart,
the check starts from the last sample of the restored flight recording.

The next 3 fixes confirm the candidate when each fix follows from the fix
before it within the same limits. Otherwise, a fix that follows from the last
accepted fix cancels the candidate. Until confirmation, the core processes the
fixes normally.

A confirmed position jump and a new flight reset all derived flight data. This
includes the wind, the circling buffer, derived airspeed, heading, turn rate,
and the vertical-speed and average-vario series. A new recording after more
than 3 hours of system time counts as a new flight. See
[Only a confirmed position jump or a new flight resets the wind](../adr/0004-only-a-confirmed-position-jump-or-a-new-flight-resets-the-wind.md).

## Pressure altitude

The current core accepts pressure altitude from a valid `$PGRMZ` value. Each
device stores its latest candidate. The first fresh candidate in external-device
order wins.

Pressure altitude is independent from GPS altitude. The two values can come
from different devices.

## Vertical speed

The core derives raw vertical speed from consecutive altitude samples. It uses
the selected pressure altitude. When no pressure altitude is current, it uses
the altitude of the selected GPS. A positive value means that the altitude is
increasing. The value is not smoothed or energy compensated.

The estimator ignores samples whose ingestion times do not advance. It starts a
new series after a pressure-source reset, a source change, or a gap longer than
30 seconds. The first sample in a series does not produce a rate.

The core also smooths raw vertical speed through two exponential stages. Each
stage has a two-second time constant fitted against recorded LXNAV LX9070 vario
values at 1 Hz. This tuning is only validated for 1 Hz pressure-altitude updates.
The second stage uses the updated first stage, so other update rates produce a
different amount of smoothing and delay.

When a current airspeed is available, the core also derives an
energy-compensated vario from the same altitude plus the airspeed energy term
`v²/2g`. It uses the selected TAS. Without a current TAS, it uses derived
airspeed. The compensated series resets when a TAS appears or its source
changes. While no current airspeed exists, the vario is unavailable.

The instruments topic retains each previous vertical-speed value with
`stale: true` while its input is stale or a new series waits for its second
sample. The vario becomes stale only with its altitude input. A stale vario
and a stale vertical speed then have the same age. The debug overlay displays
all three values with the configured vertical-speed unit.

The Flight View Vario infobox prefers the fresh vario, then the fresh smoothed
vertical speed. If both are unavailable or stale, it uses the stale vario, then
the stale smoothed vertical speed. The switch is instant. The label is "Vario"
for the vario and "Vario (unc.)" for the vertical speed, also when the value is
stale. The German label for the vertical speed is "Vario (unk.)".

## Average vario

The core calculates average vario from the selected altitude series. It uses
the height change during the previous 20 seconds. It interpolates the height at
the start of the window. During startup, it uses all available history after
the first two samples.

When a current airspeed is available at both ends of an altitude interval, the
height change includes the airspeed energy term `v²/2g`. Otherwise, the
interval uses only the altitude change. The average keeps its history when the
airspeed becomes available or unavailable. The transition does not add an
energy-height offset.

An altitude source change, reset, or gap longer than 30 seconds starts a new
average series. The instruments topic retains the previous value with
`stale: true` until the new series has two samples. The Flight View displays
this value as Avg. vario.

## Netto and relative vario

The core calculates netto from energy-compensated vario and the active polar's
sink rate. The sink rate includes ballast, bugs, ISA density, and turn load.

Relative vario (super netto) subtracts the active polar's minimum sink rate
from netto. This minimum sink rate includes ballast, bugs, and the same ISA
density estimate as netto. It uses straight flight, independent of the current
bank angle. The value estimates potential climb at minimum sink, not actual
climb while circling. Positive values mean climbing.

Both values require valid netto inputs. Before the inputs produce a value,
the fields remain absent. While no current airspeed exists, both values are
unavailable. A stale altitude input retains the previous values with
`stale: true`. A polar change updates both current values immediately.

Estimated netto does not need an airspeed. It adds the active polar's sink rate
at an assumed airspeed to the smoothed vertical speed. The assumed airspeed is
the ground speed in every flight phase. Its lower limit is the minimum-sink
airspeed, with the same ISA density correction. Without a ground speed, the
core uses the minimum-sink airspeed. The sink rate includes ballast, bugs, ISA
density, and turn load, as for netto. Estimated relative vario subtracts the
same minimum sink rate as relative vario. Both values require the vertical
speed and a polar. They become stale with the vertical speed.

The instruments topic publishes all four values. The debug overlay displays
netto and relative vario with the configured vertical-speed unit.

The Flight View Netto infobox prefers the fresh netto, then the fresh Estimated
netto. If both are unavailable or stale, it uses the stale netto, then the stale
Estimated netto. The switch is instant. The label is "Netto" for netto and
"Netto (est.)" for Estimated netto. The German label for Estimated netto is
"Netto (gesch.)".

## Wind and derived airspeed

Without an airspeed source, the core derives airspeed from the ground velocity
and the wind. The wind and the derived airspeed stay available while the wind
filter was converged after its last accepted measurement. A full circle, a
partial arc, and a TAS sample are measurements. The wind age has no limit. A new
estimate that is not converged yet, or a reset of the wind, removes both
values. With an airspeed source, the wind is available only while the variance
of the wind filter is converged. See
[Derived airspeed and wind have no age limit](../adr/0006-derived-airspeed-and-wind-have-no-age-limit.md).

The wind stays when the GPS source changes, when the selected GPS becomes
unavailable, and when the airspeed source changes. This includes a change after
the pilot disables, edits, or deletes the selected device. Only a confirmed
position jump resets the wind.

## True airspeed

The current core accepts true airspeed from `$LXWP0`. It does not calculate TAS
from IAS, altitude, or temperature. It does not ingest IAS.

TAS uses the same external-device ordering and three-second freshness boundary
as pressure altitude. The debug overlay displays the selected value with the
configured horizontal-speed unit. The Flight View TAS infobox prefers fresh
direct TAS, then fresh derived airspeed. If both are unavailable or stale, it
uses stale direct TAS, then stale derived airspeed. If neither exists, it shows
a dash. The infobox retains the selected value's freshness state.

## Frontend projection

The `Instruments` topic contains optional GPS, pressure-altitude, true-airspeed,
solar-position, and derived objects. The derived object contains optional raw,
smoothed, and energy-compensated vertical speeds, netto, and relative vario.
The topic does not publish source identity.

Canonical values cross the protocol in decimal degrees, metres, metres per
second, and milliseconds. Frontend code applies display units and locale
formatting.

The topic is a complete snapshot of the current published instrument state. The
core emits it only when that projection changes.

## Excluded behavior

The current contract does not include manual per-domain source selection,
source identity in the frontend, GPS quality presentation, IAS, additional
internal sensors or a generic source-selector framework.

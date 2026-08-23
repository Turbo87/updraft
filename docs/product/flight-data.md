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

Android GNSS supplies a complete UTC instant and ellipsoid altitude. The core
uses EGM96 to convert ellipsoid altitude to mean-sea-level altitude.

## Pressure altitude

The current core accepts pressure altitude from a valid `$PGRMZ` value. Each
device stores its latest candidate. The first fresh candidate in external-device
order wins.

Pressure altitude is independent from GPS altitude. The two values can come
from different devices.

## Vertical speed

The core derives raw vertical speed from consecutive selected pressure-altitude
samples. A positive value means that pressure altitude is increasing. The value
is not smoothed or energy compensated.

The estimator ignores samples whose ingestion times do not advance. It starts a
new series after a pressure-source reset, a source change, or a gap longer than
30 seconds. The first sample in a series does not produce a rate.

The core also smooths raw vertical speed through two exponential stages. Each
stage has a two-second time constant fitted against recorded LXNAV LX9070 vario
values at 1 Hz. This tuning is only validated for 1 Hz pressure-altitude updates.
The second stage uses the updated first stage, so other update rates produce a
different amount of smoothing and delay.

The instruments topic retains both previous vertical-speed values with
`stale: true` while the pressure altitude is stale or a new series waits for its
second sample. The debug overlay displays the raw value with the configured
vertical-speed unit. There is no pilot-facing vario display yet.

## True airspeed

The current core accepts true airspeed from `$LXWP0`. It does not calculate TAS
from IAS, altitude, or temperature. It does not ingest IAS.

TAS uses the same external-device ordering and three-second freshness boundary
as pressure altitude. The debug overlay displays the selected value with the
configured horizontal-speed unit. There is no pilot-facing TAS infobox yet.

## Frontend projection

The `Instruments` topic contains optional GPS, pressure-altitude, true-airspeed,
and derived objects. The derived object contains optional raw and smoothed
vertical speeds. The topic does not publish source identity.

Canonical values cross the protocol in decimal degrees, metres, metres per
second, and milliseconds. Frontend code applies display units and locale
formatting.

The topic is a complete snapshot of the current published instrument state. The
core emits it only when that projection changes.

## Excluded behavior

The current contract does not include manual per-domain source selection,
source identity in the frontend, GPS quality presentation, IAS, additional
internal sensors, energy-compensated vario values, netto, or a generic
source-selector framework.

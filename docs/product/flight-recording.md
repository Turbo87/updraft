# Flight recording

Status: Current behavior

Updraft records the current flight without pilot action. After a crash, an
Android process kill, or a relaunch, Updraft continues the same flight
recording.

## Recording

Each new fix of the selected source with a UTC value adds one sample. A sample
contains the UTC, the position, the MSL altitude, vario, netto, relative vario,
terrain elevation, and wind. A stale or unavailable value is empty.

A fix starts a new recording when no recording exists. A fix also starts a new
recording when its UTC is more than 3 hours after the last sample, or more than
30 seconds before it. A new recording replaces the previous recording and resets
task progress. A fix with the same UTC as the last sample, or up to 30 seconds
before it, is not recorded.

Updraft keeps only the current recording. It commits each sample in a separate
transaction. A committed sample survives an app crash and an Android process
kill. A power loss can remove the newest committed samples. A crash loses only
the samples that are not committed yet.

## Recovery

At startup, Updraft restores the recording before it connects to any device.
When the last sample is more than 3 hours older than the current time, Updraft
deletes the recording. The next fix then starts a new recording.

Otherwise, the next fix continues the restored recording. The wind of the last
sample is available directly after the restart. Without a new wind measurement,
it stays available for 30 minutes. The circling wind and the 20-second average
vario are not restored. They fill again in about 60 seconds and 20 seconds.

## Storage faults

At startup, the database can fail to open, fail its integrity check, or have a
newer version than Updraft supports. Updraft then logs a warning, deletes the
database, and starts with an empty recording. When Updraft cannot create the
database, it logs an error, restores nothing, and does not record. When it
cannot read the samples, it logs an error and restores nothing.

When Updraft cannot write a sample, it logs a warning and continues to write
the next samples. When a write succeeds again, it logs this event. The recording
then has a gap. Navigation continues. Updraft shows no warning to the pilot.

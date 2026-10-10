# Recovery replays derived samples

After a crash or an Android process kill, Updraft must rebuild the trail, task
progress, and later the flight timer. The flight recording stores one derived
sample for each fix of the selected source, and recovery replays these samples.
It stores no raw core inputs, no user actions, and no state snapshots. Derived
samples are about 10 times smaller than raw inputs and do not depend on the
core version. A rebuilt trail also shows the values that the pilot saw, after
a later polar, ballast, or bugs change.

## Considered options

- Raw core inputs reproduce every derived value and the estimator state
  exactly. Exact replay needs every input with its shell timestamp, which
  includes the 10 Hz UTC ticks, the transport read boundaries, and every
  command. That is about 1.3 MB/h for own-ship data and 2.4 MB/h with FLARM
  traffic. The developer capture of the `input-recording` roadmap item keeps
  this shape.
- Snapshots are not necessary. A replay of a 5-hour flight with a task takes
  70 ms on an Apple M2 Pro. No phone measurement exists.

## Consequences

- Recovery does not rebuild the estimator state. It seeds the wind filter from
  the wind of the last sample. The circling wind and the 20-second average vario
  fill again in about 60 s and 20 s.
- Task progress, and later takeoff and landing, are derived from the samples and
  the active route. The recording contains no user action.

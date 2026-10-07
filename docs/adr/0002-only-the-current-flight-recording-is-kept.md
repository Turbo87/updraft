# Only the current flight recording is kept

Updraft keeps one flight recording, in the SQLite database `state.sqlite`. A new
recording deletes the previous one in the same transaction. The IGC log is the
flight archive, so Updraft does not keep a second copy of past flights. The
schema therefore has no recording identifier, and recovery always continues
the one stored recording.

## Consequences

- When `state.sqlite` cannot be opened, fails `PRAGMA quick_check`, or has a
  newer schema version than the app supports, the shell deletes the database
  and starts again. Before other state moves into `state.sqlite`, for example
  the IGC log, this rule must be checked again, because it deletes that state
  too.

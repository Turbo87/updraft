# The wall clock starts a new recording, and data jumps ask the pilot

Updraft uses two clocks to find the end of a flight recording. The wall-clock
time stored with each sample shows whether the pilot was away. More than 3 h
of wall-clock time since the last sample starts a new recording automatically.
This covers a restart, a crash, and an app that was suspended overnight.

The fix UTC and the position show whether the data jumped while the app was
running. A fix-UTC jump of more than 3 h forward or more than 30 s back, or a
confirmed position jump, does not start a new recording. Updraft asks the
pilot with a blocking question that offers New flight and Continue flight. The
current recording continues until the pilot answers.

## Considered options

- A new recording on each fix-UTC jump, without a question, was the previous
  behavior. One GPS time error then deletes the real flight, because Updraft
  keeps only the current recording.
- A question also after more than 3 h away gives the pilot a question on the
  next day for a decision that is almost always the same.
- A timeout with a default answer can delete or merge a flight while the pilot
  is busy. The question has no timeout.

## Consequences

- The core owns the open question. It publishes the question in a topic and
  takes the answer as a command, so every connected client shows it and one
  answer closes it on all clients.
- The core does not store the open question. After a restart, the jump check
  compares the first new fixes with the last restored sample and asks again.
- A Main Menu "New flight" action uses the same command as the answer.

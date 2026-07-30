# NMEA Replay Server

## Purpose

Updraft needs a small development tool that replays a recorded NMEA stream.
The current application connects to external data sources as a TCP client.
The tool must therefore provide a TCP server.

The server replays the recording in real time.
Its clock does not depend on connected clients.
A client starts at the current replay position when it connects.

## Glossary

- **Anchor**: A valid RMC or GGA time that sets the replay time for nearby bytes.
- **Replay event**: One byte range and the time when the server sends it.
- **Replay schedule**: The ordered replay events for one file.

## Scope

This change provides these items:

- A new `updraft_replay` binary package in `libs/updraft_replay`.
- Replay of one NMEA file through a TCP server.
- Exact preservation of the input bytes.
- Real-time scheduling from valid RMC and GGA times.
- Playback that continues when no clients are connected.
- Simultaneous playback to multiple clients.
- `--loop` and `--skip` controls.
- A progress display.
- NMEA payload output by default.

This change does not provide these items:

- IGC parsing or replay.
- An in-app replay mode or user interface.
- NMEA sentence generation or normalization.
- Playback-speed controls.
- Historical data for a client that connects late.
- Artificial TCP write boundaries.
- Changes to the application, core, or TCP client.

## Package structure

The package has three focused modules:

- `main.rs` owns command-line options and startup.
- `replay.rs` builds the schedule and runs the replay clock.
- `server.rs` accepts clients and sends replay events.

The package uses the existing `updraft_nmea` parser.
It also uses Tokio, Clap, Tracing, Indicatif, and `tracing-indicatif`.

## Command line

The basic command is:

```bash
cargo run -p updraft_replay -- flight.nmea
```

The command interface is:

```text
updraft_replay [--listen <ADDRESS>] [--loop] [--skip <SECONDS>] <FILE>
```

`--listen` defaults to `127.0.0.1:4353`.
`--loop` restarts the recording after its final event.
`--skip` skips the selected number of seconds on the first pass only.

The tool reads and validates the file before it binds the listener.

## Byte preservation

The tool reads the complete file as bytes.
It does not require UTF-8 input.

The schedule builder scans a borrowed view with `updraft_nmea::parse()`.
It uses parsed messages only to find valid RMC and GGA times.
It does not serialize parsed messages back to NMEA.

The replay events contain contiguous ranges from the original byte buffer.
The complete event sequence contains each source byte exactly once.
TCP can split or combine writes, but the ordered byte stream stays unchanged.

These bytes remain unchanged:

- Unknown sentences.
- Sentences with invalid checksums.
- Blank lines.
- Non-UTF-8 bytes.
- An incomplete final sentence.
- Other bytes that the parser rejects.

The tool does not split output into artificial ten-byte writes.
Each client task uses `write_all()` for each replay event.

## Schedule construction

The first valid RMC or GGA time becomes replay time zero.
The schedule uses the parser's millisecond time precision.

Bytes before the first anchor join the first event.
Untimed bytes after an anchor stay with that anchor.
The next later anchor starts the next event.
Anchors with the same time stay in one event.

A backward change of more than 12 hours means that the recording crossed
midnight. The scheduler adds one day to the new time.

A smaller backward change does not move the replay clock backward.
The affected bytes use the current replay time.
The tool reports one warning for the recording.

A file without a valid RMC or GGA time is not replayable.
The tool reports an error before it binds the listener.

## Skip and loop behavior

`--skip` uses elapsed replay time.
The first pass starts with the first event at or after the requested time.
The selected event is sent without an initial delay.

A skip that is longer than the recording is an error.
The tool reports the error before it binds the listener.

`--skip` does not apply to later loops.
Each later loop starts at the first event.

With `--loop`, the next pass starts immediately after the final event.
The tool does not add a pause between passes.

Without `--loop`, the process exits after the final event.
This closes all client connections.

## Server behavior

The server binds the listener before it starts the replay clock.
The replay clock then advances even when the server has no clients.

One playback task publishes events through a bounded Tokio broadcast channel.
The task ignores the channel's no-receiver result.
This keeps playback independent of client presence.

The accept loop subscribes to the broadcast channel when it accepts a client.
It gives the subscription and socket to a new client task.
The client receives only events that the channel publishes after subscription.

Each client task writes events in order.
A disconnected client ends only its own task.
A client that falls behind the bounded channel also ends only its own task.
Neither condition changes the replay clock or another client.

The channel capacity is an internal constant.
The command line does not expose it.

## Terminal output

The tool shows one live progress line.
The line contains elapsed replay time, total duration, and estimated time
remaining.

The progress position comes from the replay clock.
Client connections do not change it.
The line resets for each loop.
The first pass starts at the selected position when `--skip` is present.

The tool logs each replayed NMEA payload by default.
The displayed payload can replace invalid UTF-8.
This display conversion does not change the transmitted bytes.

`tracing-indicatif` keeps payload and lifecycle logs above the progress line.
Lifecycle logs cover listener startup, client connections, client
disconnections, timestamp warnings, and terminal errors.

## Error behavior

These conditions stop startup with a nonzero exit status:

- The tool cannot read the file.
- The file has no valid RMC or GGA time.
- `--skip` exceeds the recording duration.
- The listener cannot bind to the requested address.

Invalid and unknown NMEA data does not stop startup.
The server sends that data unchanged.

A client write error disconnects only that client.
A lagged broadcast receiver disconnects only that client.

## Automated tests

The automated test set stays small because this package is an internal
development tool.

One schedule test covers:

- Basic event timing.
- `--skip` selection.
- Exact reconstruction of the source bytes.

One error test checks that an untimed file is rejected.

One asynchronous server test checks:

- Playback without a connected client.
- A client that connects after playback starts.
- Two simultaneous clients.

The package does not repeat parser tests from `updraft_nmea`.
It does not add a separate test for each malformed-input kind.
It does not add a dedicated slow-client test.

## Validation

The change must pass these checks:

```bash
cargo fmt --all --check
cargo test -p updraft_replay
cargo clippy -p updraft_replay --all-targets --all-features -- -D warnings
```

The Rust workspace checks from continuous integration must also stay green.

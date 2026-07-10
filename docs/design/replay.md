# Recording, Replay & Resume

Updraft uses several related mechanisms that should not be confused:

- **IGC replay** feeds recorded flight fixes into simulator mode for users, demos, and end-to-end tests.
- **Input recording** captures the exact sequence seen by the core for field debugging and saved regression tests.
- **CI recompute verification** reruns worker calculations and compares their outputs with recorded results.
- **Crash resume** restores a small set of flight-critical state and continues the IGC log.

All four depend on time and external results entering the core as data, but they have different formats and guarantees.

## Deterministic Core Replay

The core applies one input after another in a fixed order. The same ordered inputs produce the same state changes. Input recording therefore captures:

- normalized sensor observations and commands,
- monotonic clock advancement,
- external I/O results,
- compute-worker results.

Replay applies those inputs exactly as recorded. It never calls an external service or reruns a worker. A field failure can be repeated without reproducing thread schedules, network timing, or the platform math library.

Worker results may use most of the recording space. They live in a compressed companion file next to the main input log. The companion file may be left out when storage is limited. In that case, replay cannot reproduce worker-backed values exactly.

## Determinism Scope

Replay is bit-for-bit exact for the same build on the same platform. On another platform, replay still applies exactly the same recorded inputs. Tests use tolerances for floating-point values that may differ slightly between platforms.

Recording results rather than recomputing them avoids making platform-specific `sin`, `cos`, and `atan2` behavior part of the replay contract. It also keeps a recording useful after an algorithm changes.

A recording reproduces the behavior that was recorded. It is not a promise that a later build would have scheduled the same worker jobs or produced the same outputs from the original raw inputs.

## CI Recompute Verification

CI may replay a recorded flight while rerunning worker jobs in their original order. It hashes each result and compares it with the recorded result. The first mismatch reports the input position and worker kind.

This mode detects unstable results and planned behavior changes. It is a test tool, not the normal replay method. Stateful workers rebuild their caches by receiving jobs in the same order.

Workers still use stable iteration order and fixed calculation order where practical. This keeps saved tests repeatable without requiring identical bits on every platform.

## IGC Replay and Simulator Mode

IGC replay is a user feature built on the simulator input path. It parses fixes and feeds typed observations to the core at a selected speed. It bypasses device framing because an IGC file contains flight data, not captured device bytes.

Raw byte-capture replay is a separate developer tool. It feeds recorded bytes through the real framing, checksum, parser, normalization, and device-detection path.

Simulator and IGC replay disable live sensors, IGC recording, weather downloads, and OGN traffic. Seeking or moving the simulated aircraft creates an explicit position jump. The app can then reject worker results and cached calculations from before the jump.

## Crash Resume

A client subscription snapshot and a crash-resume snapshot are unrelated types:

- The client snapshot contains current shared display state and active sets. It is cheap to generate and safe to resend on every subscription.
- The resume snapshot contains only flight-critical state needed after process death, such as the active task, logging status, and device configuration.

Resume snapshots have an explicit format version. They are never a serialized copy of the whole `App`. If an app update cannot read the version, it discards the resume snapshot and starts with fresh state. The IGC log remains the trusted flight record and can still continue on its own.

Snapshot and IGC writes are non-blocking effects executed by the runtime's I/O adapter. A failed resume-snapshot write becomes a user-visible warning. Input recording is also written incrementally, so a crash leaves behind the sequence needed to reproduce it.

Replay tooling supports both replay from an empty `App` and replay from a resume snapshot plus an input-log position.

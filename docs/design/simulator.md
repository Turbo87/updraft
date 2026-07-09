# Simulator & Replay

Simulator and replay are the third data source category next to external devices and internal sensors (see [devices.md](devices.md)). While simulator or replay mode is active, the other two source categories are disabled: `App` receives positions, altitudes, and vario values exclusively from the simulator.

Both modes produce normalized observations through the same application input path as internal sensors. No device protocol is involved, so the parser is bypassed. Replaying recorded raw device bytes through the real parser is a separate developer tool (see [devmode.md](devmode.md)), while deterministic application-input replay is covered in [core.md](core.md).

Activating simulator or replay mode sets application and client flags that disable functionality which must not run against simulated data, including IGC writes and live weather or OGN loading.

## Simulator Mode

Simulator mode is activated from within the app. It provides basic flying controls and the ability to set location and altitude directly.

## Replay Mode

Replay mode is a variant of the simulator that drives playback from an IGC file instead of manual controls: play/pause, skipping time forward and backward, and adjustable replay speed.

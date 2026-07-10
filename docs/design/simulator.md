# Simulator & Replay

Simulator and replay are the third data source category next to external devices and internal sensors (see [devices.md](devices.md)). While simulator or replay mode is active, the other two source categories are disabled: the core receives positions, altitudes, and vario values exclusively from the simulator.

Both modes produce typed messages directly onto the core's input queue, like internal sensors do. No device protocol is involved, so the parser is bypassed. (Replaying recorded raw device bytes through the real parser is a separate developer tool, see [devmode.md](devmode.md); deterministic input-sequence replay for debugging is covered in [core.md](core.md).)

Activating simulator or replay mode sets a flag in the core and frontend that disables functionality which must not run against simulated data: the IGC file writer, but also live weather and OGN data loading/display.

## Simulator Mode

Simulator mode is activated from within the app. It provides basic flying controls and the ability to set location and altitude directly.

## Replay Mode

Replay mode is a variant of the simulator that drives playback from an IGC file instead of manual controls: play/pause, skipping time forward and backward, and adjustable replay speed.

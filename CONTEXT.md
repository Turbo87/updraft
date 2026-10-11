# Updraft

Updraft is a cross-platform soaring flight computer. This glossary defines the project terms.

## Language

**AAT**:
Assigned Area Task (competition task type with area turnpoints).

**AGL**:
height Above Ground Level.

**Aircraft preset**:
a read-only, built-in catalogue entry for an aircraft type (polar, masses, wingspans, handicap). Not a device config.

**Aircraft profile**:
a specific user aircraft (registration, callsign) created from a preset or from scratch, with per-field overrides. Only one active at a time.

**Airspeed source**:
a device that supplies measured true or indicated airspeed.
_Avoid_: TAS input, TAS source

**BLE**:
Bluetooth Low Energy.

**Client**:
a frontend connected to the core (local Tauri webview, browser, or secondary client on another phone/tablet). Never a connected instrument.

**CUP**:
SeeYou waypoint/task file format.

**Derived airspeed**:
true airspeed that Updraft calculates from ground velocity and wind when no airspeed source supplies it.
_Avoid_: inferred airspeed, estimated airspeed

**Device**:
a connected instrument (GPS, vario, FLARM). Never a phone/tablet running the app, which is a client.

**Emergency mode**:
the navigation-target mode that shows up to three suitable landables and highlights the one currently selected.

**Engine noise level (ENL)**:
a 0 to 999 measure of engine noise, as defined by the IGC specification, from a device or the microphone of the phone or tablet.

**Engine running**:
the state that Updraft derives from the engine noise level.

**Estimated netto**:
netto from vertical speed and the polar sink at ground speed, used when no measured or derived airspeed exists.
_Avoid_: netto fallback, uncompensated netto

**Estimated relative vario**:
estimated netto minus the minimum sink of the polar.

**FLARM**:
collision-warning system ubiquitous in European gliding.

**FL (Flight Level)**:
pressure altitude in hundreds of feet referenced to 1013.25 hPa.

**Flight recorder**:
the component that records the current flight into a flight recording.

**Flight recording**:
the stored, time-ordered fixes of one flight, with the values derived for each fix, produced by the flight recorder. Not a developer capture of core inputs.

**Flight View**:
the primary in-flight UI containing the map, Situation Bar, infoboxes, and flight controls.

**Focused target**:
the one active navigation target that currently drives guidance, calculations, and target-dependent infoboxes.

**GPS-only**:
a setup whose only flight-data input is GPS. It has no pressure altitude, no airspeed source, and no other sensor input.

**IGC file**:
FAI-standardized flight log format.

**Infobox**:
a configurable cell that shows flight information or provides a small, focused control. Its content is not limited to navigation.

**MacCready**:
speed-to-fly theory parameter (expected climb rate).

**Map inspector**:
the UI opened by tapping the map to inspect a selected position and any nearby objects.

**Main Menu**:
the full-screen root for screens outside the Flight View, opened by the Flight View's Menu control.

**NMEA**:
serial sentence protocol used by GPS and avionics devices, extended by vendors with proprietary `$P...` sentences.

**OGN**:
Open Glider Network, crowd-sourced ground-station network for glider position tracking via FLARM.

**OpenAir**:
de-facto standard airspace text format.

**PMTiles**:
single-file, HTTP-range-readable tile archive format.

**Pinned target**:
a navigation target kept visible in a compact readout below the Situation Bar whenever it is not focused.

**Position jump**:
a confirmed change of position of the selected GPS that no glider can fly.

**QNE**:
standard pressure reference (1013.25 hPa). basis for flight levels.

**QNH**:
local sea-level pressure setting. makes the altimeter read altitude AMSL.

**Reach polygon**:
area reachable in glide from current position, terrain-aware.

**Screen**:
a full-screen destination opened from Main Menu, such as Devices or Flight Review.

**Situation Bar**:
the fixed Flight View region that normally shows the focused target and is temporarily replaced by the highest-priority warning.

**SPP**:
Bluetooth Serial Port Profile (classic Bluetooth serial).

**Task progress**:
the reached task points and the start and finish times, derived from the flight recording and the route, independent of the navigation target.

**Trail**:
the map rendering of the current flight recording. Also called snail trail.

**Vario**:
the total-energy vertical speed. It excludes altitude changes that come from airspeed changes. Also called compensated vario.
_Avoid_: TE vario

**Vertical speed**:
the rate of altitude change without energy compensation. Also called uncompensated vario.

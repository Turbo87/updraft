# Main Menu and Screens

Main Menu is the stable entry point to everything outside the Flight View. It
opens fullscreen without discarding the map position, focused target, infobox
page, or temporary flight state behind it.

## Launch Behavior

Cold-start behavior remains an open prototype decision:

- Open the Flight View directly.
- Show a preflight dashboard when no flight is active.
- Show a lightweight preflight overlay over the Flight View.

A preflight surface may select the aircraft, task, device setup, and recording
state before flight. It is a preparation surface, not the primary app
navigation. If a flight is already active, the app resumes the Flight View
directly.

Direct Flight View startup is the smallest initial implementation. The data
model must still avoid assuming that the last aircraft is always correct.

## Main Menu

Main Menu begins with a **Current flight** section. It summarizes the active
aircraft, task and focused target, flight and recording state, active sensor
sources, and connection or warning problems. It provides context without
becoming another instrument panel.

The stable destinations are:

- Plan
- Flights
- Aircraft
- Devices
- Data & Maps
- Display & Infoboxes
- Settings

They remain visible and retain their order during flight. Contextual **In
flight** shortcuts may appear above them, but they do not hide or rearrange the
complete app.

Every screen header contains Back, Map, its title, and local actions. Back
follows the current screen hierarchy. Map returns directly to the preserved
Flight View. An active warning replaces the Map icon with a warning icon and
applies the warning severity color without changing the control's action or
size. Collision warnings also appear directly below the header as defined in
[warnings.md](warnings.md). Opening Main Menu from the Flight View always shows
its root.

## Screen Responsibilities

**Plan** is map-centered. It combines the task route with an ordered point list
and task calculations. On phones, adding or editing a point opens a focused
full-screen chooser or editor. Wide displays keep the list and map visible
together.

**Flights** begins with a searchable chronological log. Flight details move from
summary to map replay, statistics, and analysis without placing everything on
one screen.

**Aircraft** manages aircraft profiles, identity, polar, handicap, ballast, and
related settings.

**Devices** manages configured sources, connections, priority, and diagnostics.
Its interaction model is defined in [devices.md](devices.md).

**Data & Maps** manages offline regions and datasets. It shows download state,
installed version, freshness, size, and update problems. Wide displays may pair
the catalogue with a map preview.

**Display & Infoboxes** lists every configured page, including Thermal, and
opens any page in its layout editor. It also manages page order and saved
layouts.

**Settings** contains stable application-wide preferences in grouped categories
with search. Device, aircraft, display, and data configuration remain in their
own screens rather than becoming one deep settings tree.

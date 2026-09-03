# Documentation

This index is the entry point for Updraft documentation. Documents have one of
four roles. Do not combine the roles when you interpret a requirement.

## Current system

- [Application architecture](architecture.md) defines the current ownership and
  integration boundaries.
- [Implementation roadmap](roadmap.md) records delivered and planned work.
- [Product scope](product-scope.md) classifies target, partial, excluded, and
  undecided capabilities without implying delivery status.
- [CSS](css.md) defines current frontend styling rules.
- [Glossary](glossary.md) defines project terms.

Code and tests define implemented behavior. A document can define accepted
behavior that is not implemented yet only when it states that status.

## Product design

Product documents define accepted user-visible behavior. A product document
must state whether its behavior is current or planned.

- [Settings](product/settings.md) defines current navigation, ownership,
  persistence, and presentation behavior.
- [External devices](product/devices.md) defines supported connections,
  lifecycle, ordering, and Settings behavior.
- [Flight data](product/flight-data.md) defines source priority, freshness,
  selection, and frontend projection.
- [Airspace](product/airspace.md) defines import, canonical data, storage,
  resources, and current map presentation.
- [Basemap](product/basemap.md) defines offline Enroute files, tile lookup,
  zoom limits, and attribution.
- [Terrain](product/terrain.md) defines offline elevation tiles, hillshade,
  and source attribution.
- [Traffic](product/traffic.md) defines FLARM observation, identity, freshness,
  topic updates, map presentation, and details.
- [Flight View](product/flight-view.md) defines the map session, position follow
  mode, and map inspection.

## Research

Research documents record dated observations and analysis. They inform product
decisions but do not define Updraft behavior.

- [Research index](research/README.md) defines the evidence and freshness rules.
- [Ecosystem research](research/ecosystem/README.md) contains dated capability
  inventories for related navigation systems and avionics.

- [Technical investigations](research/investigations/README.md) contain dated
  root-cause evidence for resolved or constrained problems.

## Verification evidence

Verification documents record manual or physical checks that automated tests
cannot prove. They describe the tested environment, result, and limitations.

- [Verification records](verification/README.md) contain dated manual and
  physical test results.

## Development guides

- [Android platform](development/android.md) defines the current build,
  foreground-session, permission, GNSS, SPP, and activity boundaries.
- [Replay server](development/replay.md) defines the current NMEA and IGC TCP
  replay tool for development and manual testing.
- [Testing](development/testing.md) defines test ownership, local checks, and
  the limits of each test layer.

# Documentation

This index is the entry point for Updraft documentation. Documents have one of
four roles. Do not combine the roles when you interpret a requirement.

## Current system

- [Application architecture](architecture.md) defines the current ownership and
  integration boundaries.
- [Implementation roadmap](roadmap.md) records delivered and planned work.
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
- [Traffic](product/traffic.md) defines FLARM observation, identity, freshness,
  topic updates, map presentation, and details.

The documents under `design/` contain older product and architecture designs.
Some sections describe superseded architecture. They are historical material,
not a source for current system boundaries.

## Research

Research documents record dated observations and analysis. They inform product
decisions but do not define Updraft behavior.

The current research material is under `discovery/` and
`superpowers/investigations/`. The product research will use the
`research/ecosystem/` location.

## Verification evidence

Verification documents record manual or physical checks that automated tests
cannot prove. They describe the tested environment, result, and limitations.

The current verification records are under `superpowers/verification/`.

## Development guides

- [Android platform](development/android.md) defines the current build,
  foreground-session, permission, GNSS, SPP, and activity boundaries.

## Historical specifications

The dated documents under `superpowers/specs/` record earlier design and
delivery decisions. Newer documents sometimes supersede individual sections of
older documents. These files preserve history. They do not override the current
architecture or implemented behavior.

Active requirements from these specifications belong in the current
architecture, product, or development documents.

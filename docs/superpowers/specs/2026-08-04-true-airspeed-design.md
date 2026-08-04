# True airspeed

## Context

The NMEA parser reads true airspeed from the LXWP0 sentence. The core does not
use this value. The frontend cannot show it.

This design adds true airspeed as an independent flight-data domain. It sends
the selected value to the debug overlay.

## Relationship to the flight-data source-selection specification

This specification extends
[`2026-08-04-flight-data-source-selection-design.md`](2026-08-04-flight-data-source-selection-design.md).
It supersedes only the exclusion of TAS ingestion and source selection in that
specification. All other requirements remain active.

TAS means true airspeed in this specification.

## Scope

This design includes these functions:

- Read TAS from the parsed LXWP0 sentence.
- Store one TAS candidate for each external device.
- Select TAS independently from GPS and pressure altitude.
- Publish TAS and its state in the `Instruments` topic.
- Show TAS and its state in the debug overlay.
- Format TAS with the configured speed unit.

This design does not include these functions:

- IAS ingestion or display.
- TAS calculation from IAS, altitude, or temperature.
- TAS from an internal sensor or a sentence other than LXWP0.
- Other LXWP0 values, such as pressure altitude, vario, heading, or wind.
- Source identity in the frontend.
- A generic source-selector abstraction.
- A cockpit infobox or other pilot-facing TAS display.

## Input processing

Each successfully parsed LXWP0 sentence can contain one optional TAS value. The
parser represents the value as a `Speed`. It converts the LXWP0 value from
kilometers per hour.

The core accepts a present TAS value. It stores the value with the ingestion
time of the LXWP0 sentence. A repeated value refreshes its ingestion time.

An absent or invalid TAS field does not change the candidate. It also does not
refresh the candidate ingestion time. Other LXWP0 fields do not update a
flight-data domain in this slice.

## Source selection

TAS is independent from GPS and pressure altitude. A source can provide TAS
while another source provides GPS or pressure altitude.

The core applies the external-device order. The first enabled source with a
fresh TAS candidate wins. A fresh higher-priority source immediately replaces
a lower-priority source.

TAS uses the fixed 3-second freshness limit. A candidate is fresh while its age
is less than 3 seconds. A candidate becomes stale when its age is 3 seconds or
more.

The selected TAS becomes last known when no fresh candidate exists. The core
keeps the last selected value and its original ingestion time. A new fresh
candidate makes TAS current again.

Device reorder keeps TAS candidates and selects them in the new order. Device
disable, edit, and delete clear the candidate for the affected source. If the
selected source is reset, the core selects a fresh fallback. TAS becomes
unavailable when no fallback exists.

The implementation uses a TAS-specific selection function. It does not add a
generic selector, policy trait, or callback.

## Instruments topic

The `Instruments` topic adds an optional `trueAirspeed` object:

```ts
type TrueAirspeedInstruments = {
  metersPerSecond: number;
  stale: boolean;
};
```

The value is `null` when TAS is unavailable. A current value has `stale: false`.
A last-known value has `stale: true`.

The topic keeps the canonical value in meters per second. It does not contain a
formatted value or a display unit.

## Debug overlay

The debug overlay adds these rows after ground speed:

- **True airspeed** shows the selected TAS value.
- **True airspeed state** shows `Current`, `Stale`, or `Unavailable`.

The value uses the configured speed unit. It shows one decimal place and the
unit label. An unavailable value shows `–`.

The overlay is a developer tool. This design does not add TAS to a cockpit
infobox or another pilot-facing view.

## Automated tests

Implementation follows red-green-refactor.

Core tests cover these behaviors:

- A present LXWP0 TAS value selects TAS for its source.
- An absent TAS field does not update or refresh the candidate.
- A repeated TAS value refreshes the candidate.
- TAS selects independently from GPS and pressure altitude.
- TAS follows source priority, fallback, freshness, and last-known rules.
- Reorder and source reset apply the existing domain rules.

Protocol tests cover unavailable, current, and stale TAS output. The generated
TypeScript bindings contain the new instruments type and field.

Frontend tests cover unavailable, current, and stale TAS. They also cover the
configured speed unit.

## Acceptance criteria

This feature is complete when all these statements are true:

- A valid LXWP0 TAS value enters the core as a typed speed.
- TAS uses independent source selection and the 3-second freshness limit.
- Missing LXWP0 TAS does not clear or refresh a candidate.
- The `Instruments` topic publishes canonical TAS and stale state.
- The debug overlay shows TAS with the configured speed unit.
- The debug overlay shows the TAS domain state.
- The implementation does not ingest other LXWP0 values.

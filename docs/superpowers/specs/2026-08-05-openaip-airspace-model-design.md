# OpenAIP airspace data model

## Context

Updraft parses local OpenAir files into a canonical airspace model. The current
model retains only the fields that the first map display needed. Its class and
type values also use OpenAir strings.

The [OpenAIP airspace schema](https://api.core.openaip.net/api/schemas/response/airspace/airspace-schema.json)
defines a broader aviation data model. It uses numeric values for airspace type,
ICAO class, and activity. It also defines vertical limits, operational flags,
frequencies, transponder settings, operating hours, activation dates, and
remarks.

Updraft will align its canonical airspace model with the aviation fields in the
OpenAIP schema. The OpenAir adapter will populate every field that the source
format and the `openair` crate expose. The GeoJSON projection will continue to
contain only the fields that MapLibre needs.

## Relationship to the local OpenAir specification

This specification extends
[`2026-08-02-openair-airspace-design.md`](2026-08-02-openair-airspace-design.md).
It supersedes only these parts of that specification:

- The canonical fields and classification rules in **Parser adapter**.
- The location and output contract of the projection in **Ownership** and
  **GeoJSON resource**.
- The class and type values in **Map rendering**.
- The related parser, URI, frontend, and acceptance requirements.

All other requirements remain active. This includes complete-file rejection,
polygon normalization, the 1 metre curve-error limit, core state, storage,
startup, commands, resource transport, Settings, and layer ordering.

## Scope

This design includes these functions:

- Define the canonical OpenAIP aviation fields in `updraft_airspace`.
- Use OpenAIP numeric values for airspace type, ICAO class, and activity.
- Convert OpenAir values to the canonical model when a valid mapping exists.
- Retain absence when OpenAir does not provide a value.
- Project the canonical model to a rendering-only GeoJSON subset.
- Use OpenAIP property names and numeric values in that subset.
- Keep MapLibre style expressions readable through named constants.

This design does not include these functions:

- Import OpenAIP JSON or GeoJSON files.
- Serialize the complete canonical model.
- Expose non-rendering fields to the frontend.
- Add airspace warnings, filters, labels, selection, or details.
- Change polygon normalization or the 1 metre curve-error limit.
- Add OpenAIP service or audit metadata.
- Create globally stable airspace identifiers.

## Sources of truth

The OpenAIP airspace schema is the source of truth for field meaning and numeric
values. Updraft uses the values that the schema publishes at the time of this
specification.

The canonical model includes aviation data. It does not include OpenAIP service
data. These fields remain excluded:

- `_id`.
- `dataIngestion`.
- `deletable`.
- `createdBy` and `updatedBy`.
- `createdAt` and `updatedAt`.

The OpenAir format and the `openair` crate remain the sources of truth for the
OpenAir adapter. The adapter does not parse raw OpenAir records again after the
dependency has produced `openair::Airspace`.

## Ownership

The `updraft_airspace` crate owns the canonical airspace types, the OpenAir
adapter, polygon normalization, and `Airspace::to_geojson()`.

The `updraft_core` crate owns the active dataset state. It does not depend on
GeoJSON or `serde_json`. The Tauri resource obtains an immutable dataset
snapshot and serializes the resulting features outside the driver task.

Keeping `Airspace::to_geojson()` in `updraft_airspace` puts the projection next
to the model that it reads. The method is a rendering projection. It is not
general serialization for the canonical model.

## Canonical airspace

Each `Airspace` contains these fields:

- `id: AirspaceId`.
- An optional name.
- A required `AirspaceType`.
- A required `AirspaceClass`.
- An optional `AirspaceActivity`.
- Optional `on_demand`, `on_request`, `by_notam`, `special_agreement`, and
  `request_compliance` values.
- One canonical polygon.
- Optional validated country codes.
- A required lower limit and upper limit.
- An optional minimum lower limit and maximum upper limit.
- Zero or more frequencies.
- Zero or more transponder settings.
- Optional hours of operation.
- Optional activation start and end dates.
- Optional remarks.

An optional boolean distinguishes an absent source value from an explicit
`false` value. The adapter does not replace absence with `false`.

Collections are empty when the source provides no entries. Values that have a
format or range invariant use validated domain types. Raw source strings do not
remain in the canonical model when a more precise type can enforce the
invariant.

## Dataset-local identifier

`AirspaceId` keeps its current representation and serialization. It is a
zero-based sequence number in parser order. It is unique only within one parsed
dataset.

A repeated parse of the same ordered source produces the same identifiers. An
identifier can refer to a different airspace after source replacement.

MapLibre scopes feature identifiers to a source. Updraft uses the source ID
`airspace`. The numeric identifier therefore does not need an `airspace:`
prefix. Updraft does not currently attach feature state to airspaces. A future
feature-state design must clear state when the active dataset changes or define
stable identifiers.

## Airspace type

`AirspaceType` is required. It uses these OpenAIP values:

- `0`: Other.
- `1`: Restricted.
- `2`: Danger.
- `3`: Prohibited.
- `4`: Controlled Tower Region.
- `5`: Transponder Mandatory Zone.
- `6`: Radio Mandatory Zone.
- `7`: Terminal Maneuvering Area.
- `8`: Temporary Reserved Area.
- `9`: Temporary Segregated Area.
- `10`: Flight Information Region.
- `11`: Upper Flight Information Region.
- `12`: Air Defense Identification Zone.
- `13`: Airport Traffic Zone.
- `14`: Military Airport Traffic Zone.
- `15`: Airway.
- `16`: Military Training Route.
- `17`: Alert Area.
- `18`: Warning Area.
- `19`: Protected Area.
- `20`: Helicopter Traffic Zone.
- `21`: Gliding Sector.
- `22`: Transponder Setting.
- `23`: Traffic Information Zone.
- `24`: Traffic Information Area.
- `25`: Military Training Area.
- `26`: Control Area.
- `27`: ACC Sector.
- `28`: Aerial Sporting or Recreational Activity.
- `29`: Low Altitude Overflight Restriction.
- `30`: Military Route.
- `31`: TSA or TRA Feeding Route.
- `32`: VFR Sector.
- `33`: FIS Sector.
- `34`: Lower Traffic Area.
- `35`: Upper Traffic Area.
- `36`: Military Controlled Tower Region.

The Rust enum exposes an exact numeric conversion for GeoJSON. It does not
retain an unsupported OpenAir type string. A source type without an OpenAIP
mapping becomes `Other`.

## ICAO class

`AirspaceClass` is required. It uses these OpenAIP ICAO class values:

- `0`: Class A.
- `1`: Class B.
- `2`: Class C.
- `3`: Class D.
- `4`: Class E.
- `5`: Class F.
- `6`: Class G.
- `8`: Unclassified or special-use airspace.

The type name and GeoJSON property name remain `AirspaceClass` and `class`.
The `class` property uses the numeric values from the OpenAIP `icaoClass`
field.

## Activity

`AirspaceActivity` is optional. It uses the documented numeric values in the
OpenAIP schema:

- `0`: No specific activity.
- `1`: Parachuting.
- `2`: Aerobatics.
- `3`: Aeroclub and aerial work.
- `4`: Ultra-light machine activity.
- `5`: Hang gliding or paragliding.

The schema enum also permits code `6`, but its description does not define that
value. Updraft treats code `6` as unknown. An OpenAIP adapter must log a warning
when it encounters code `6` or another unsupported activity value. It then maps
the value to `NoSpecificActivity`, which has numeric value `0`.

OpenAir does not provide this structured field through the current parser. The
adapter leaves it absent.

## Operational flags

The canonical model defines these optional OpenAIP flags:

- `on_demand`.
- `on_request`.
- `by_notam`.
- `special_agreement`.
- `request_compliance`.

OpenAir does not provide these structured fields through the current parser.
The adapter leaves each value absent. An airspace type whose name mentions
NOTAM does not set `by_notam` implicitly.

## Country and remarks

Country values use validated ISO 3166-1 alpha-2 codes. The model supports one
or multiple countries. It normalizes the OpenAIP scalar-or-array representation
to one canonical collection. An absent source value produces no country codes.

Remarks are optional text. OpenAir does not expose canonical airspace remarks
through the current parser. The adapter leaves both fields absent.

## Vertical limits

The primary vertical fields are named `lower_limit` and `upper_limit`. They are
required.

The model keeps the existing semantic `AirspaceAltitude` representation. It
supports:

- Ground.
- Altitude above mean sea level.
- Height above ground level.
- Flight level.
- Unlimited.

The first four forms correspond to OpenAIP combinations of value, unit, and
reference datum. `Unlimited` remains an Updraft extension because OpenAir can
contain that value and the OpenAIP schema has no unlimited vertical-limit
variant.

`lower_limit_min` is an optional hard minimum for the lower limit. It supports
rules such as a height above ground that must never fall below an altitude
above mean sea level.

`upper_limit_max` is an optional hard maximum for the upper limit. It supports
rules such as a variable upper limit that must never exceed a specified
altitude.

OpenAir has one `AL` record and one `AH` record. It cannot represent the two
additional constraints. The adapter leaves them absent.

## Frequencies

Each frequency contains:

- A validated frequency value.
- The OpenAIP MHz unit with numeric value `2`.
- An optional name.
- An optional primary value.
- Optional remarks.

The validated value can format the OpenAIP `ddd.ddd` representation. The model
does not use a raw string as proof that the value has this format.

The OpenAir adapter maps one frequency and its optional call sign to one
canonical frequency. It uses the call sign as the frequency name. It marks the
entry as primary. It leaves remarks absent.

An invalid frequency is a conversion error. It rejects the complete source as
required by the existing import design.

## Transponder settings

Each transponder setting contains:

- A validated four-digit octal code.
- A required primary value.
- Optional remarks.

The OpenAir adapter maps one transponder code to one primary setting. It adds
leading zeroes when they are necessary for the four-digit representation. It
leaves remarks absent.

A code that contains a digit outside `0` through `7` is a conversion error. It
rejects the complete source.

## Operating hours and activation dates

Hours of operation contain one or more day-specific operating periods and
optional remarks. The operating-period type represents only the six shapes
that the OpenAIP schema permits:

- A fixed start and fixed end.
- A fixed start and sunset end.
- A sunrise start and fixed end.
- A sunrise start and sunset end.
- No explicit time or sun marker.
- Activation by NOTAM.

Each period contains its day of week, public-holiday exclusion, and optional
remarks. Fixed times use a time-of-day type. The sum type does not permit
contradictory combinations of fixed times, sunrise, sunset, and NOTAM flags.

`active_from` and `active_until` use `time::OffsetDateTime`. Each boundary is
optional.

The current `openair` dependency parses activation dates into
`ActivationTimes`, but version 0.5.0 does not expose its start and end fields.
The adapter leaves both canonical fields absent. It does not parse debug output
or serialize and reparse the dependency type.

## OpenAir classification adapter

The adapter always produces one `AirspaceType` and one `AirspaceClass`.

Modern `AC` values `A` through `G` map to the matching class. `Unclassified`
maps to OpenAIP class value `8`.

A supported `AY` value maps to its OpenAIP type. If no supported `AY` mapping
exists, a legacy type in `AC` provides the type. Otherwise the type is `Other`.
This includes a missing `AY`, `AY NONE`, an empty value that the dependency
accepts, and an unknown value. The adapter discards the unsupported raw code.

The adapter uses these direct `AY` mappings:

- `ACCSEC` to ACC Sector.
- `ADIZ` to Air Defense Identification Zone.
- `ALERT` to Alert Area.
- `ASRA` to Aerial Sporting or Recreational Activity.
- `ATZ` to Airport Traffic Zone.
- `AWY` to Airway.
- `CTA` to Control Area.
- `CTR` to Controlled Tower Region.
- `FIR` to Flight Information Region.
- `FIS` to FIS Sector.
- `GSEC` to Gliding Sector.
- `HTZ` to Helicopter Traffic Zone.
- `LTA` to Lower Traffic Area.
- `MATZ` to Military Airport Traffic Zone.
- `MTA` to Military Training Area.
- `MTR` to Military Training Route.
- `OFR` to Low Altitude Overflight Restriction.
- `P` to Prohibited.
- `Q` to Danger.
- `R` to Restricted.
- `RMZ` to Radio Mandatory Zone.
- `TIA` to Traffic Information Area.
- `TIZ` to Traffic Information Zone.
- `TMA` to Terminal Maneuvering Area.
- `TMZ` to Transponder Mandatory Zone.
- `TRA` to Temporary Reserved Area.
- `TRAFR` to TSA or TRA Feeding Route.
- `TSA` to Temporary Segregated Area.
- `UIR` to Upper Flight Information Region.
- `UTA` to Upper Traffic Area.
- `VFRSEC` to VFR Sector.
- `WARNING` to Warning Area.

OpenAir values without an equivalent OpenAIP type map to `Other`. This includes
`CUSTOM`, `N`, `TFR`, `TRZ`, `VFRR`, and unknown values. OpenAir `TFR` means a
temporary flight restriction. It must not map to OpenAIP value `31`, which
means a TSA or TRA feeding route.

Legacy type values in `AC` map as follows:

- `Ctr` to Controlled Tower Region.
- `Restricted` to Restricted.
- `Danger` to Danger.
- `Prohibited` to Prohibited.
- `WaveWindow` to Gliding Sector.
- `RadioMandatoryZone` to Radio Mandatory Zone.
- `TransponderMandatoryZone` to Transponder Mandatory Zone.
- `GliderProhibited` to Other.

Each legacy type uses `Unclassified` as its required ICAO class.

## Other OpenAir fields

The adapter keeps the existing mappings for name, vertical limits, and polygon
geometry. Geometry normalization remains unchanged.

The adapter leaves activity, operational flags, country, vertical limit
constraints, hours of operation, activation dates, and remarks absent. It does
not infer these values from names or type strings.

The adapter validates a frequency or transponder setting before it constructs
the canonical value. A conversion error contains the dataset-local airspace ID
and rejects the complete file.

## GeoJSON projection

`Airspace::to_geojson()` returns a rendering-only GeoJSON feature. It stays in
`updraft_airspace`.

The feature has this contract:

```json
{
  "type": "Feature",
  "id": 0,
  "properties": {
    "type": 4,
    "class": 3
  },
  "geometry": {
    "type": "Polygon",
    "coordinates": [
      [
        [10.0, 50.0],
        [10.1, 50.0],
        [10.0, 50.1],
        [10.0, 50.0]
      ]
    ]
  }
}
```

The top-level `id` is the numeric dataset-local `AirspaceId`. It is not an
OpenAIP `_id` and is not globally stable. The `properties` object does not
contain another `id` property.

The `type` property uses the OpenAIP name and numeric values. The `class`
property keeps the existing Updraft name and uses the numeric values from the
OpenAIP `icaoClass` field. These are the only canonical properties that the
current map renderer needs. The projection omits all other fields.

The method documentation links to the official OpenAIP airspace schema. It
states that the two classification properties use numeric values from that
schema.

The dataset resource still returns a GeoJSON FeatureCollection. Resource URL,
content type, cache headers, empty-collection behavior, and snapshot ownership
remain unchanged.

## Map rendering

The MapLibre component reads numeric `type` and `class` properties. It does not
read the previous OpenAir string values.

The layer component defines named constants for every numeric value that its
style expressions use. Style expressions do not contain unexplained numeric
airspace codes.

Type styling keeps precedence over class styling. The initial semantic groups
use these OpenAIP types:

- Controlled: Controlled Tower Region, Terminal Maneuvering Area, Airport
  Traffic Zone, Airway, Control Area, ACC Sector, and Military Controlled Tower
  Region.
- Prohibited, restricted, and danger: Restricted, Danger, Prohibited,
  Protected Area, and Low Altitude Overflight Restriction.
- Mandatory zones: Transponder Mandatory Zone, Radio Mandatory Zone, and
  Transponder Setting.
- Gliding and recreation: Gliding Sector and Aerial Sporting or Recreational
  Activity.
- Other: unmatched types.

ICAO classes A through E use the controlled style when no matching type takes
precedence. Classes F, G, and Unclassified use the other style.

Colors, opacity, widths, layer ordering, and the active-dataset behavior remain
unchanged.

## Tests

Implementation follows red-green-refactor. Each independently reviewable field
addition includes its focused model or adapter test.

Airspace model and adapter tests cover these behaviors:

- Each classification enum returns the exact OpenAIP numeric value.
- Every parsed airspace has a required type and ICAO class.
- Supported modern and legacy OpenAir values map to the expected OpenAIP value.
- Missing, `NONE`, empty, and unsupported types map to `Other` when no legacy
  mapping applies.
- Unsupported raw type codes do not remain in the canonical model.
- Each field that OpenAir cannot supply remains absent.
- Minimum and maximum vertical constraints remain absent.
- A valid frequency and call sign produce one primary frequency.
- A valid transponder code produces one primary setting.
- Invalid frequency and transponder values reject the complete source.
- Existing polygon and altitude behavior remains unchanged.

GeoJSON tests cover these behaviors:

- The dataset-local ID is the top-level Feature `id`.
- `properties` contains only numeric `type` and `class` values.
- The previous `properties.id` and OpenAir string values are absent.
- Polygon closure and FeatureCollection behavior remain unchanged.

Frontend tests use fixtures with numeric OpenAIP values. They verify the
existing layer behavior.

## Acceptance criteria

This migration is complete when all these statements are true:

- `updraft_airspace::Airspace` contains all OpenAIP aviation fields in scope.
- OpenAIP service and audit fields are absent from the canonical model.
- Every airspace has an `AirspaceType` and `AirspaceClass`.
- Type, ICAO class, and documented activity values retain their OpenAIP numeric
  meaning.
- Unknown or unsupported OpenAir type values become `Other`.
- Fields that OpenAir cannot represent remain absent instead of receiving
  invented defaults.
- OpenAir frequency, call sign, and transponder values populate validated
  canonical entries.
- Activation dates remain absent until the parser exposes their values.
- `AirspaceId` retains its current numeric representation and becomes the
  top-level GeoJSON Feature ID.
- Rendering GeoJSON contains only numeric `type` and `class` properties.
- The GeoJSON method documents the OpenAIP values in that subset.
- MapLibre expressions use named constants for OpenAIP values.
- Existing storage, state, geometry, error, Settings, and rendering behavior
  remains active unless this specification explicitly changes it.

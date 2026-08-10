# Waypoints

Status: Planned. The canonical data model exists in `libs/updraft_waypoint`.
No importer, storage, or presentation exists.

A waypoint is one named point of interest. This document defines the
canonical model only. Import, storage, map presentation, and search come with
the features that use the model.

## Sources

The model represents two sources.

The SeeYou CUP format supplies one comma-separated record for each waypoint.
The columns are `name`, `code`, `country`, `lat`, `lon`, `elev`, `style`,
`rwdir`, `rwlen`, `rwwidth`, `freq`, `desc`, `userdata`, and `pics`. Column
order is free. Columns after `style` can be absent. Elevation and runway
values carry a unit suffix. Country values use IANA top-level domains.

OpenAIP supplies seven non-airspace datasets: airports, navaids, obstacles,
hotspots, reporting points, hang gliding sites, and RC airfields. Each record
is one GeoJSON `Point`. Each record has a name, a country, an MSL elevation,
and remarks. Each dataset adds its own type value and its own attributes.
Country values use ISO 3166-1 alpha-2 codes and can be an array.

Every record of both sources is one point. One point model covers all of
them.

## Canonical model

`Waypoint` holds the attributes that all sources supply:

- `id`, a sequence number within the parsed dataset
- `name` and optional `code`
- `country_codes` as unvalidated source text
- `position`
- `elevation`, optional, above mean sea level
- `kind`
- `description`, from the CUP `desc` column or the OpenAIP remarks

The `WaypointId` value is stable only for its dataset. It is not durable
across a source replacement. This follows the `AirspaceId` rule.

The model applies no country registry. It keeps the source text. This follows
the airspace country rule. The CUP and OpenAIP code sets differ, for example
`UK` against `GB`.

## Kinds

`WaypointKind` is the union of the CUP styles and the OpenAIP point types. A
collapsed kind cannot be recovered without a new import, so the model keeps
every source distinction.

A variant carries a payload when its source family supplies attributes that
no other family uses. Families with the same attribute set share one payload.
Every landing site type shares `Airfield`, so no payload is repeated.
Variants without further attributes stay simple.

A kind records what a point is. It does not record independent attributes of
that point. The CUP styles and the OpenAIP airport types mix five axes: the
form, the surface, the operator, the use, and the status. The model separates
them:

- `AirfieldType` holds the form. The form says what a pilot lands on.
- `RunwaySurface` holds the surface.
- `operators` holds the civil or military category. A site can have both,
  because joint civil and military aerodromes exist.
- `uses` holds the stated use, for example gliding.
- `closed` holds the status.

The form has six values, because a pilot lands on six different things: a
prepared aerodrome, an outlanding field, a simple landing strip, a sloped
mountain altiport, a water surface, and a heliport. A civil aerodrome and a
military aerodrome are the same thing to land on. A glider site and an
ultralight site are prepared aerodromes with a stated user group. An
agricultural landing strip is a landing strip with a stated user group.

The use does not restrict a landing by itself. The runway data, the
`prior_permission_required` flag, and the `private_use` flag state the
restrictions.

### CUP styles

- 0 Unknown becomes `Unknown`.
- 1 Waypoint becomes `Waypoint`.
- 2 Airfield with grass surface runway becomes `Airfield(Aerodrome)` with a
  grass runway composition.
- 3 Outlanding becomes `Airfield(Outlanding)`.
- 4 Gliding airfield becomes `Airfield(Aerodrome)` with the gliding use.
- 5 Airfield with solid surface runway becomes `Airfield(Aerodrome)` with a
  solid runway composition.
- 6 Mountain Pass becomes `MountainPass`.
- 7 Mountain Top becomes `MountainTop`.
- 8 Transmitter Mast becomes `Obstacle(TransmitterMast)`.
- 9 VOR becomes `Navaid(Vor)`.
- 10 NDB becomes `Navaid(Ndb)`.
- 11 Cooling Tower becomes `Obstacle(CoolingTower)`.
- 12 Dam becomes `Dam`.
- 13 Tunnel becomes `Tunnel`.
- 14 Bridge becomes `Bridge`.
- 15 Power Plant becomes `PowerPlant`.
- 16 Castle becomes `Castle`.
- 17 Intersection becomes `Intersection`.
- 18 Marker becomes `Marker`.
- 19 Control/Reporting Point becomes `ReportingPoint`.
- 20 PG Take Off becomes `HangGlidingSite(TakeOff)`.
- 21 PG Landing Zone becomes `HangGlidingSite(Landing)`.

A transmitter mast and a cooling tower are vertical obstructions, so both
become obstacles. A power plant, dam, tunnel, bridge, castle, intersection,
and marker are landmarks, so each keeps its own kind.

The CUP `rwdir`, `rwlen`, and `rwwidth` columns become one runway without a
designator. Styles 2 and 5 supply the composition of that runway. A record
with a surface style but no runway columns still becomes one runway, because
the composition needs a runway to hold it. The `freq` column becomes one
airfield frequency without a purpose.

CUP names no material for a solid surface, so `RunwayComposition` has a
`Solid` value. The OpenAIP compositions name a material.

### OpenAIP datasets

- Airports become `Airfield`. The 14 source types become six forms, the
  operators, the uses, and the closed flag:
  - Types 0, 3, and 9 become `Aerodrome`.
  - Type 2 becomes `Aerodrome` with the civil operator.
  - Type 5 becomes `Aerodrome` with the military operator.
  - Type 8 becomes `Aerodrome` with the closed flag.
  - Type 1 becomes `Aerodrome` with the gliding use.
  - Type 6 becomes `Aerodrome` with the ultralight use.
  - Type 4 becomes `Heliport` with the military operator.
  - Type 7 becomes `Heliport` with the civil operator.
  - Type 10 becomes `WaterAirfield`.
  - Type 11 becomes `LandingStrip`.
  - Type 12 becomes `LandingStrip` with the agricultural use.
  - Type 13 becomes `Altiport`.
- Navaids become `Navaid`, with one `NavaidType` for each of the nine source
  types.
- Obstacles become `Obstacle`, with one `ObstacleType` for each of the five
  source types.
- Hotspots become `Hotspot`.
- Reporting points become `ReportingPoint`.
- Hang gliding sites become `HangGlidingSite`.
- RC airfields become `RcAirfield`.

OpenAIP airport type 1 and CUP style 4 both become an aerodrome with the
gliding use. OpenAIP navaid types 2 and 3 receive the same kinds as CUP
styles 10 and 9. The other values of both sources stay separate.

The model does not keep the international label of airport type 3 or the IFR
label of type 9. Both state the scale and the procedures of an aerodrome, not
its form. The traffic types, the IATA code, the frequencies, the instrument
approach aids, and the runway dimensions record the same facts with more
detail. This is the only source distinction that the model drops.

## Kind-specific attributes

- `Airfield` keeps the form, the operators, the uses, the closed flag, the
  ICAO, IATA, and alternate identifiers, the traffic types, the magnetic
  declination, the prior-permission, private, skydive, and winch-only flags,
  the services, the frequencies, the runways, and the hours of operation. A
  runway keeps its designator, direction, operations, turn direction,
  surface, dimensions, declared distances, threshold location, permitted
  aircraft types, lighting, and approach aids.
- `Navaid` keeps the identifier, channel, frequency, range, magnetic
  declination, true-north alignment, and hours of operation.
- `Obstacle` keeps the height above ground.
- `Hotspot` keeps the reliability, the occurrence, the aircraft categories,
  the times of day, and the favorable and required wind directions.
- `HangGlidingSite` keeps the wing categories, the access ways, the
  certification, the suitable wind directions, and the take-off direction.
- `ReportingPoint` keeps the compulsory flag and the airport references.
- `RcAirfield` keeps the operator, the permitted engine types, the permitted
  altitude, and the hours of operation.

Physical values use the typed `updraft_units` quantities. Frequencies keep an
exact three-digit decimal value with its unit, so two values compare exactly.

## Excluded source values

The model excludes service and provenance values:

- OpenAIP document identifiers and audit stamps
- OpenAIP `OpenStreetMap` references and import stamps
- OpenAIP images, contact values, and telephone services
- OpenAIP `elevationGeoid` values, because `updraft_egm96` owns geoid
  separation
- the CUP `userdata` and `pics` columns

The airspace model excludes the same class of values.

The CUP task section is not part of this model. Tasks reference waypoints by
name. A task model comes with the task feature.

## Open decisions

- The model has no OpenAIP numeric codes. Each importer owns the conversion
  from its own wire values. A GeoJSON projection needs the numeric codes and
  can add them, as the airspace model does.
- `OperatingHours` and the frequency value duplicate the equivalent airspace
  types. A shared crate can hold one copy when the first importer lands.
- The runway lighting, approach aid, declared distance, and passenger
  facility values have no current consumer. The model keeps them because the
  sources supply them.
- The product relevance of RC airfields is undecided.

## Verification limit

The attribute inventory uses the
[OpenAIP Core API schemas](https://api.core.openaip.net/api/system/specs/v1/schema.json).
The published GeoJSON export files were not inspected, because the export
bucket needs an account key. An importer must confirm the exported property
names against a real file.

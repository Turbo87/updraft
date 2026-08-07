# Map Inspector Design

## Purpose

The pilot needs to inspect airspace and traffic at a selected map position. A
normal map tap opens one full-screen result page. MapLibre selects the matching
features from dedicated hit layers.

This is the first map-inspector slice. It does not depend on waypoints or
navigation targets.

## Superseded guidance

This specification supersedes only the first-slice map-inspector guidance in
these documents:

- The **Map Inspector** section in
  [`docs/design/ui/flight-view.md`](../../design/ui/flight-view.md).
- The initial dependency and result-order guidance for map-inspector items in
  [`docs/roadmap.md`](../../roadmap.md).

This specification also supersedes these GeoJSON requirements:

- The rendering-only projection and `class` property in
  [`2026-08-05-openaip-airspace-model-design.md`](2026-08-05-openaip-airspace-model-design.md).
- The exclusion of country values from GeoJSON in
  [`2026-08-05-openaip-airspace-country-values.md`](2026-08-05-openaip-airspace-country-values.md).

All other requirements in those documents remain active. This includes the
canonical airspace model, OpenAir import, storage, resource transport, current
airspace styling, and dataset-local identifiers.

## Scope

This design includes these functions:

- Open `/nearby` after each normal map tap.
- Select airspace and traffic through MapLibre hit layers.
- Show the selected coordinates, distance, and bearing.
- Show separate airspace and traffic result lists.
- Keep the initial traffic membership while results are open.
- Update selected traffic from later traffic messages.
- Open independent airspace and traffic detail routes.
- Include all canonical airspace metadata in GeoJSON.
- Show explicit loading, empty, error, unavailable, and not-found states.
- Show traffic hit areas through the map debug overlay.

This design does not include these functions:

- Navigation targets or point actions.
- CUP files, waypoints, airports, or landables.
- Task points, terrain, weather, or markers.
- A Rust spatial query or map-inspection command.
- Result sorting or duplicate removal.
- Changes to visible airspace styling.
- Airspace vicinity results or vertical filtering.
- Multiple active airspace files or durable airspace identity.
- Responsive sheets or side panels.
- A generic inspector result framework.
- New traffic protocol fields.

## Ownership

`updraft_airspace` owns the canonical airspace model and its GeoJSON
projection. `updraft_core` continues to own the active airspace dataset and
traffic state. The Tauri resource continues to serve the airspace GeoJSON
snapshot.

The frontend owns map selection and presentation. The persistent Flight View
owns the MapLibre instance and the hit layers. Route pages use the shared map,
airspace, traffic, instruments, and settings state.

This feature does not add a core input, Tauri command, or generated protocol
type for map inspection.

## Route structure

The feature uses these routes:

- `/nearby/[latitude]/[longitude]` shows inspection results.
- `/airspaces/[id]` shows one airspace.
- `/traffic/[id]` shows one traffic target.

Latitude and longitude use decimal degrees. The traffic ID occupies one encoded
path segment. The airspace ID is the current dataset-local numeric ID.

The routes do not contain a hit radius or timestamp. MapLibre owns the fixed
traffic hit radius. A new route instance repeats the query against the current
map state.

Airspace URLs are not durable across an airspace-file replacement. A replaced
dataset can assign the same ID to a different airspace.

All three routes cover the complete Flight View. The Flight View and map stay
mounted below the route content.

## Map interaction

A normal MapLibre click opens `/nearby` for the clicked coordinate. The route
opens when no feature matches. A map control activation or a pan does not open
the route. Existing MapLibre interaction handling decides whether an event is
a click.

The map click does not pass selected features to the route. The `/nearby` page
waits for the required sources and layers. It projects the URL coordinate to a
screen point. It then calls `queryRenderedFeatures()` for the airspace and
traffic hit layers at that point.

The query uses only content that MapLibre currently renders. The page does not
move the map to an off-screen URL coordinate. Such a coordinate therefore has
no MapLibre matches. Direct visits and reloads are exceptional but use this
same process. Browser Back mounts a new `/nearby` page and repeats the query.

## Hit layers

### Airspace

The `airspace` source contains one separate `airspace-hit` fill layer. The layer
covers every airspace polygon. It has constant zero fill opacity and no visible
outline.

The hit layer does not replace or modify a visible airspace layer. Existing
colors, opacities, widths, semantic groups, and the relative order of visible
layers remain unchanged.

### Traffic

The `traffic` source contains one circle hit layer. Each circle has a radius of
24 CSS pixels. Device pixel density does not change this radius. The layer
includes every current traffic target and does not use symbol collision state.

The circles have zero opacity by default. The map debug overlay adds a
**Traffic hit areas** checkbox. The checkbox makes the circles visible. It does
not change their radius or selection behavior.

### Query result

The page uses the feature sequence that MapLibre returns for each category. It
does not sort the features. It does not remove duplicate features. Airspace and
traffic remain separate result categories.

Each airspace result uses the feature properties from MapLibre. Each traffic
result uses the top-level feature ID to find the current target in the traffic
store.

## Airspace GeoJSON

`Airspace::to_geojson()` returns one complete frontend projection. The output
is still one GeoJSON Feature. The top-level `id` remains the numeric
dataset-local `AirspaceId`. The feature geometry remains the canonical polygon.

The `properties` object uses the names and value shapes from the
[OpenAIP airspace schema](https://api.core.openaip.net/api/schemas/response/airspace/airspace-schema.json)
when the schema can represent the canonical value. It can contain these
properties:

- `name`.
- `type`.
- `icaoClass`.
- `activity`.
- `onDemand`.
- `onRequest`.
- `byNotam`.
- `specialAgreement`.
- `requestCompliance`.
- `country`.
- `upperLimit`.
- `lowerLimit`.
- `upperLimitMax`.
- `lowerLimitMin`.
- `frequencies`.
- `transponderSettings`.
- `hoursOfOperation`.
- `activeFrom`.
- `activeUntil`.
- `remarks`.

The projection omits an absent optional scalar or object. It includes
`frequencies` and `transponderSettings` as arrays. A single country value is a
string. Multiple country values are an array. An empty country collection is
absent. Raw country values keep their canonical text and order.

Frequency, transponder, operating-hours, and date values use the OpenAIP names
and shapes. Date values use RFC 3339 text. The projection does not add OpenAIP
service or audit fields. It omits `_id`, `dataIngestion`, `deletable`,
`createdBy`, `updatedBy`, `createdAt`, and `updatedAt`.

### Vertical limits

A representable vertical limit uses the OpenAIP `value`, `unit`, and
`referenceDatum` properties:

- Ground uses value `0`, metre unit `0`, and ground datum `0`.
- MSL altitude uses a whole-foot value, feet unit `1`, and MSL datum `1`.
- AGL height uses a whole-foot value, feet unit `1`, and ground datum `0`.
- Flight level uses its level value, flight-level unit `6`, and standard datum
  `2`.

The current OpenAir adapter produces integral feet and flight levels. The
projection keeps those input values when it converts the canonical units.

OpenAIP has no unlimited-limit value. Updraft represents an unlimited limit as
this documented extension:

```json
{
  "unlimited": true
}
```

The frontend accepts an OpenAIP limit object or this Updraft extension.

### Map style compatibility

The existing map style reads `icaoClass` instead of `class`. This property-name
change does not change the style values or visible output. Named numeric
constants remain in the layer component.

## Source readiness

The shared frontend state records when the initial airspace and traffic topics
arrive. A route does not report an empty or not-found state before the related
initial state is available.

The `/nearby` page waits for the MapLibre style and traffic hit layer. When an
airspace dataset is active, it also waits for the airspace source and hit layer.
No active airspace dataset is a valid state. The page can still show traffic.

The page keeps the selected feature sequence after the query completes. An
airspace source replacement does not change the mounted result page. The query
uses whichever airspace source MapLibre renders when it first becomes ready. A
remount runs a new query.

## Nearby page

The page shows the selected coordinates independently from the result lists.
When ownship position is available, it also shows current distance and true
bearing from ownship to the selected point. The frontend uses Turf to calculate
these values. Svelte `$derived` values recalculate them when ownship moves. The
page applies the configured distance unit.

The page shows two categorized lists:

- **Airspaces**.
- **Traffic**.

One result remains a one-item list. An empty query shows that no airspace or
traffic is at the selected position.

An airspace row identifies the airspace with its name, type, ICAO class, and
vertical limits. It omits an absent name. A traffic row identifies the traffic
type and shows a user-facing form of its canonical ID.

Each row is a link to its independent detail route.

## Live traffic results

The initial MapLibre query fixes the traffic IDs and their sequence for the
lifetime of the mounted `/nearby` page. The page keeps one local target snapshot
for each ID. It subscribes to the existing traffic store while mounted.

A later update replaces the local snapshot for a matching ID. A new ID does not
enter the list. A matching target stays in the list when it moves outside the
original hit circle.

A later removal does not remove the row. The page keeps the last complete
snapshot and marks it unavailable. A later update for the same ID replaces the
snapshot and makes the row available again.

If the initial MapLibre feature ID does not exist in the traffic store, the
page keeps the ID and shows an unavailable row.

## Airspace detail page

The `/airspaces/[id]` route supports direct visits and reloads. It waits for the
initial airspace state and the MapLibre `airspace` GeoJSON source. It calls
`GeoJSONSource.getData()` and finds the feature with the requested top-level
ID. It does not fetch the GeoJSON resource separately.

The page shows all canonical metadata that the feature contains. It always
shows type, lower limit, and upper limit. It shows ICAO class unless the
airspace is unclassified. It omits absent optional fields and empty sections.
It uses user-facing enum labels and the configured display units where a
conversion applies.

An inactive dataset or missing feature produces an airspace-not-found state.

## Traffic detail page

The `/traffic/[id]` route supports direct visits and reloads. It waits for the
initial traffic state. It then reads the current target from the traffic store.

The page shows these values:

- Canonical ID.
- Traffic type.
- Current coordinates.
- Optional absolute MSL altitude.
- Optional true ground track.
- Alarm level.
- Fresh, stale, or unavailable state.
- Distance and true bearing from current ownship position.

The frontend derives distance and bearing when ownship and traffic positions
are available. These values update when either position changes. The page uses
the configured distance and altitude units.

The route keeps its latest complete target while mounted. A removal marks the
retained target unavailable. A later update for the same ID replaces the
retained value and makes it available again.

A direct visit shows a traffic-not-found state when the current traffic state
does not contain the requested ID. A retained unavailable result row remains a
normal detail link and therefore shows the same not-found state.

## Navigation and accessibility

The `/nearby` page provides a Back to map control. Each detail page provides a
Back control that uses browser history. Each detail page also provides a Map
control for a direct visit.

The result categories use headings and lists. Each result is an accessible
link. Loading, empty, error, unavailable, invalid-route, and not-found states
use text. Color is not the only state indicator.

All visible text uses Paraglide messages. Numeric values use tabular digits
where this improves scanning.

## Failure behavior

An invalid latitude or longitude shows an invalid-selection state with a Back
to map control. The page does not query MapLibre. Coordinates must be finite.
Latitude must be from -90 through 90. Longitude must be from -180 through 180.

The nearby airspace category reports no matching airspace when no dataset is
active, the dataset is unavailable, or MapLibre cannot render the airspace
source. It does not provide Retry. An unexpected exception from `project()` or
`queryRenderedFeatures()` remains an application error.

An airspace source read failure on the independent detail route shows an
explicit error with Retry. A successful source read with no matching ID shows
the not-found state. These states are different.

## Tests

Implementation follows red-green-refactor. Tests stay at the layer that owns
each behavior.

Rust GeoJSON tests use an Insta snapshot for one airspace that contains every
canonical metadata field. Focused tests cover absent optional values, raw
country values, the unlimited extension, and the dataset-local feature ID.

Chromium map tests use real MapLibre. They verify these behaviors:

- The transparent airspace hit layer returns a containing feature.
- The traffic hit layer returns a target within 24 CSS pixels.
- The traffic hit layer does not return a target outside 24 CSS pixels.
- The debug option changes hit-circle visibility but not query results.
- The new airspace hit layer does not change visible airspace styling.

Pure TypeScript tests cover coordinate parsing, Turf calculation boundaries,
MapLibre feature interpretation, value formatting, metadata sections, and
retained traffic snapshots. These tests use plain values and stores. They do
not mock SvelteKit routes or MapLibre.

Playwright tests use the real application with deterministic airspace and
traffic fixtures. They cover these behaviors:

- Each normal map tap opens the expected `/nearby` URL.
- Delayed initial state, no dataset, unavailable data, empty results, and
  populated results show the correct state.
- Overlapping airspace and traffic keep their MapLibre sequence in each
  category.
- An off-screen direct `/nearby` URL does not move the map and has no MapLibre
  matches.
- Later airspace changes do not replace an already selected feature sequence.
- Direct airspace and traffic detail URLs work after reload.
- Traffic values update and remain after removal.
- Browser Back remounts `/nearby` and repeats the query.
- Invalid URLs and missing features show explicit states.

The tests do not add route component tests. Such tests would require brittle
MapLibre and SvelteKit mocks.

## Acceptance criteria

This feature is complete when all these statements are true:

- Every normal map tap opens `/nearby/[latitude]/[longitude]`.
- A map tap with no result opens an explicit empty state.
- MapLibre selects airspace and traffic from dedicated hit layers.
- Airspace hit testing includes the complete polygon interior.
- Traffic hit testing uses a fixed radius of 24 CSS pixels.
- The debug overlay can show traffic hit circles.
- The result page preserves the MapLibre feature sequence without changes.
- Nearby airspace results use one snapshot of currently rendered map content.
- An off-screen direct nearby URL does not move the map.
- Missing, unavailable, or unrendered airspace produces an empty result without
  Retry.
- Airspace GeoJSON contains all canonical metadata.
- OpenAIP property names and shapes apply where they can represent the value.
- Documented Updraft extensions preserve other canonical values.
- Visible airspace styling does not change.
- Traffic membership stays fixed while `/nearby` remains mounted.
- Matching traffic values update without adding new targets.
- Removed matching traffic remains visible as unavailable.
- Direct airspace details read the complete MapLibre GeoJSON source.
- Independent airspace and traffic routes support direct visits and reloads.
- Airspace details show all available canonical metadata.
- Traffic details show all current published fields.
- The implementation adds no Rust spatial query or inspector protocol.
- The implementation adds no airports, waypoints, navigation, sorting, or
  duplicate removal.

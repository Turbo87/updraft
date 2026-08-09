# Implementation Roadmap

Status: Delivery backlog

The [current application architecture](architecture.md) uses a deterministic
Rust core and a Tauri shell. The frontend sends commands through Tauri IPC. The
core sends topics through one Tauri channel. MapLibre reads large resources
through `updraft://` URLs.

An HTTP API is not part of the current architecture. It can return later if a specific feature needs it. Multi-display support is a far-future, optional feature.

- **Start with one complete feature path.** A feature uses only the core state, shell adapters, resource paths, or frontend presentation that it needs.
- **Replay is development infrastructure.** The replay tool sends recorded NMEA or converted IGC data through the same TCP and NMEA path as a device.
- **Harden each parser when it lands.** Each parser has no-panic tests and snapshots for applicable recorded data in `testdata/`.
- **Treat this document as a rough plan.** Each feature needs a focused design
  before implementation.

A checked item means that the described slice exists in the current code. It
does not mean that the broader product capability is complete. An unchecked
item is backlog, not an accepted design or delivery commitment. Use
[`product-scope.md`](product-scope.md) for product intent and the product
documents for accepted behavior.

## MVP delivery status

- [x] **core-and-shell** — TCP bytes flow through the NMEA decoder and deterministic core to Tauri topics and the ownship map.
- [x] **android-platform** — the foreground service, partial wake lock, internal GNSS, process survival, webview rebuild, and screen wake behavior work on Android.
- [x] **devices-and-traffic** — persisted TCP and Android Bluetooth SPP devices feed the core. Basic FLARM targets appear on the map.
- [x] **airspace-milestone** — the app imports one local OpenAir file, keeps canonical airspace in the core, and serves GeoJSON through `updraft://`.
- [x] **locale-selection** — persist one locale selection and apply it to the frontend.
- [x] **unit-selection** — persist display units and apply them to current flight and traffic values.
- [x] **external-device-management** — persist TCP and Bluetooth SPP devices. Add, edit, enable, disable, and delete them in Settings.
- [x] **airspace-source-management** — import, replace, and remove one local OpenAir source in Settings.
- [x] **map-inspection** — open a nearby route from a map point and show current
  airspace and traffic results with detail routes.
- [ ] **map-orientation-setting** — persist map orientation and add its Settings control. _(needs: map-orientation, settings-persistence)_
- [ ] **flight-data-fields** — add a fixed-slot Flight View dock for the first altitude, speed, direction, and time values. _(needs: route-shell, frontend-protocol, units-settings)_
- [ ] **offline-basemap** — import one local MBTiles basemap and serve its vector tiles through `updraft://localhost/basemap/`.

## Scaffolding

- [x] **workspace** — Cargo workspace, rustfmt/clippy config, MIT/Apache-2.0 license files, CI workflow (fmt, clippy, test).
- [x] **frontend-scaffold** — SvelteKit + Svelte 5 + TypeScript skeleton in `frontend/`, Vitest component-test setup, lint/format config, CI job.
- [x] **tauri-scaffold** — Tauri shell for desktop and Android, with development and production build workflows. _(needs: frontend-scaffold)_

## Core skeleton

- [x] **units** — custom newtype quantities (length/altitude, speed, vertical speed, angle to start; pressure, mass, temperature added when features need them), conversions, and unit-system formatting. Start minimal and grow. _(needs: workspace)_
- [x] **geo** — lat/lon types, WGS84 distance/bearing/destination-point (via `geographiclib-rs`) with a haversine fast path, bounding boxes with antimeridian handling, `geo-types` interop behind a feature. Coordinate parsing/formatting is out of scope: each data-format crate parses its own wire format, display formatting is a UI concern. _(needs: units)_
- [x] **egm96** — `libs/updraft_egm96`: EGM96 geoid undulation lookup (`separation`, ellipsoidal↔MSL helpers) via a bilinear 1° grid downsampled from the official 15′ `WW15MGH` source, with a feature-gated `downsample` generator and golden test. Used to convert bare-ellipsoidal GNSS altitude to MSL (and back for IGC). _(needs: geo)_
- [x] **core-app** — `Core::apply()` accepts one typed input and a shell-supplied timestamp. It returns typed responses and effects. The core owns topics and read-only snapshots. It has no I/O, clock, thread, Tokio, or Tauri dependency. _(needs: units)_
- [x] **core-time** — the shell supplies monotonic timestamps and a fixed 10 Hz tick. Scenario tests supply exact timestamps and do not read a wall clock. _(needs: core-app)_
- [x] **tauri-driver** — one shell task owns the core, the tick, subscribers, transport workers, and settings persistence. A new subscriber first receives the current value of every topic. _(needs: core-app, core-time)_
- [ ] **compute-workers** — run expensive pure calculations outside the core update path. Use one active job and one conflated pending job per kind. Reject stale results by generation. _(needs: tauri-driver)_

## Shell, protocol, and walking skeleton

- [x] **frontend-protocol** — generated TypeScript topic types, one typed client interface, direct Tauri commands, one Tauri topic channel, frontend stores, and a browser fake. _(needs: frontend-scaffold, tauri-driver)_
- [x] **frontend-map** — MapLibre map with the interim OpenFreeMap basemap and manual pan and zoom. _(needs: frontend-scaffold)_
- [x] **map-position** — core state drives the ownship symbol. Map sources read large data through URLs instead of the topic channel. _(needs: frontend-map, frontend-protocol)_
- [x] **route-shell** — the root layout keeps the Flight View and map state alive below route content. Settings use dedicated routes with explicit return navigation. _(needs: map-position, frontend-protocol)_
- [ ] **app-shell** — add the Main Menu, common screen headers, and responsive phone and wide-screen navigation. _(needs: route-shell)_
- [x] **resource-scheme** — the Tauri shell serves role-based resources through `updraft://localhost/`. Airspace GeoJSON is the first resource. _(needs: tauri-scaffold, tauri-driver)_
- [x] **e2e-scaffold** — Playwright uses the browser fake and a minimal inline map style. Tests can send topics, inspect shared map state, and wait for deterministic MapLibre results. _(needs: map-position, frontend-protocol)_
- [x] **map-inspector** — every normal map tap opens a coordinate route with
  distance and bearing from ownship. The route queries current MapLibre hit
  layers without moving the camera. _(needs: route-shell, frontend-map)_

## Sensor input & replay

- [x] **nmea** — `libs/updraft_nmea`: the line-based text parser — framing, checksum, resync, the always-decode structure, and generic GNSS (GGA/RMC/GSA) plus the cross-device `$PGRMZ` baro-altitude sentence, into typed structs. Vendor families land as sibling slices. _(needs: units, geo)_
- [x] **lx-nmea** — LXNav sentences (`$LXWP0-4`, `$PLXV*`) as an `updraft_nmea` slice: baro altitude, IAS/TAS, TE vario, wind, settings read/write. _(needs: nmea)_
- [x] **openvario-nmea** — OpenVario/XCVario `$POV` sentence (pressure, airspeed, TE vario) as an `updraft_nmea` slice. _(needs: nmea)_
- [x] **cambridge-nmea** — Cambridge `!w` vario records as an `updraft_nmea` slice. _(needs: nmea)_
- [x] **connection-ingestion** — the core owns one decoder and one observation set for each external device. TCP and SPP transports send bytes through this common path. _(needs: nmea, core-time)_
- [x] **tcp-client** — the Tauri shell maintains TCP client connections with reconnect backoff. It provides the desktop development path. _(needs: connection-ingestion, tauri-driver)_
- [x] **source-selection** — ordered external devices supply GPS, pressure altitude, and true airspeed. Fresh values use the first eligible source. Internal GNSS is the final GPS fallback. Values become stale after three seconds. _(needs: connection-ingestion, internal-gnss)_
- [ ] **gps-status** — retain fix quality and satellite information. Add a user-visible source and fix-status indicator. _(needs: source-selection)_
- [ ] **io-detection-and-corrections** — add passive capability observation, optional framing selection, manual overrides, and per-device corrections when a concrete device needs them. _(needs: connection-ingestion)_
- [x] **developer-replay** — `updraft_replay` sends NMEA files or converted IGC data through a TCP server in real time. It supports skip and loop controls. _(needs: nmea, tcp-client)_
- [ ] **igc-read** — add reusable application-level IGC parsing for the records and extensions that future product features need. _(needs: units, geo)_
- [ ] **replay** — add in-app replay at variable speed for simulator mode and demos. It sends typed simulator inputs and does not act as a device. _(needs: igc-read, core-time)_
- [ ] **input-recording** — optionally record the exact core input sequence in `captures/`. Save worker results in a compressed companion file. Replay can start from an empty core or a saved resume snapshot. _(needs: replay, compute-workers)_
- [ ] **flight-modes** — detect takeoff, landing, cruise, and circling. Publish the flight timer and current mode. _(needs: source-selection)_
- [ ] **vario-values** — TE/netto/relative vario, integrator and thermal averagers computed in core from GPS + baro inputs. _(needs: nmea, flight-modes)_

## Glide computer

- [x] **polar** — glide polar model (quadratic coefficients, ballast/bugs degradation), a starter polar library, speed-to-fly and MacCready ring math. _(needs: units)_
- [ ] **glide-settings** — MacCready, ballast, bugs, safety heights / safety MC: commands, state, and a settings dialog. _(needs: polar, core-app, frontend-protocol)_
- [ ] **wind-circling** — wind estimation from circling drift; wind vector in state, manual override command, wind display. _(needs: flight-modes)_
- [ ] **wind-zigzag** — airspeed-based zigzag/EKF wind estimation, layered wind statistics, source blending. _(needs: wind-circling, lx-nmea)_
- [ ] **final-glide** — wind-corrected arrival altitude for an arbitrary target (Mc and Mc-0), safety-height aware. _(needs: glide-settings, wind-circling)_
- [ ] **speed-to-fly** — STF / speed command values, dolphin speed, auto MacCready modes. _(needs: glide-settings, vario-values)_
- [ ] **infobox-values** — add tap panels and searchable quick replacement for the fixed flight-data fields. Replacement preserves the slot. _(needs: flight-data-fields)_
- [ ] **thermal-assistant** — climb sampling around the circle, centering aid view, thermal profile (climb vs altitude band). _(needs: vario-values)_
- [ ] **thermal-history** — own-climb thermal markers on the map with wind drift compensation. _(needs: thermal-assistant, wind-circling, frontend-map)_
- [ ] **density-altitude** — pressure/density-altitude tools, potential-temperature trigger aid. _(needs: lx-nmea)_

## Waypoints & navigation

- [ ] **cup** — `libs/updraft_cup`: SeeYou CUP waypoint/task file parser (CUPX and other formats come later). _(needs: units, geo)_
- [ ] **waypoint-db** — core waypoint store: multiple files, landable distinction, search, nearest-N queries. _(needs: cup, core-app)_
- [ ] **file-import** — import files through an OS picker or share intent. Route each file to the matching store by type. _(needs: waypoint-db, tauri-scaffold)_
- [ ] **cupx** — SeeYou CUPX waypoint files (CUP plus embedded images). _(needs: cup)_
- [ ] **openaip-waypoints** — OpenAIP airport/waypoint parser. _(needs: waypoint-db)_
- [ ] **gpx-waypoints** — GPX waypoint parser. _(needs: waypoint-db)_
- [ ] **geojson-waypoints** — GeoJSON waypoint parser. _(needs: waypoint-db)_
- [ ] **dat-waypoints** — Cambridge DAT waypoint parser. _(needs: waypoint-db)_
- [ ] **wpt-waypoints** — Winpilot/CompeGPS WPT waypoint parser. _(needs: waypoint-db)_
- [ ] **waypoints-on-map** — waypoint/landable symbology, labels, and zoom-dependent declutter. _(needs: waypoint-db, frontend-map)_
- [ ] **navigation-targets** — direct-to navigation with one focused target and zero or more additional targets representing waypoints or arbitrary map positions in one ordered sequence. Switching focus updates guidance, distance and ground-track-relative bearing, target-dependent infoboxes, and the course line without discarding the other targets. _(needs: waypoint-db, infobox-values)_
- [ ] **pinned-navigation-targets** — optional, unlimited target pins rendered in a content-sized area below the Situation Bar, ordered with the navigation sequence and sharing its target-list action. Focused targets appear only once. _(needs: navigation-targets)_
- [ ] **map-inspector-waypoints** — a point-first inspector that opens on every normal map tap, always shows distance and point actions beginning with **Navigate here**, and lists nearby waypoints and landables even for one result. Add fullscreen categorized result lists on phones and waypoint details such as elevation, runway, frequency, and notes. This establishes the extensible inspector result model. _(needs: waypoints-on-map, navigation-targets)_
- [ ] **arrival-heights** — reachability of landables via final glide; arrival-height labels and reachability colouring. _(needs: final-glide, waypoints-on-map)_
- [ ] **emergency-navigation** — Emergency target mode with up to three ranked reachable landables, including a suitable airfield when available. Preserve the selected candidate, update the other two, draw and label every route, and allow direct map selection. _(needs: arrival-heights, pinned-navigation-targets)_
- [ ] **nearest-lists** — sortable nearest waypoint/landable/airfield list pages. _(needs: arrival-heights)_
- [ ] **ga-routes** — GA flight-route editor (leg-based, distinct from scored tasks). _(needs: waypoint-db, frontend-map)_
- [ ] **vnav** — VNAV to altitude constraints. _(needs: final-glide, navigation-targets)_

## Terrain

- [ ] **dem** — `libs/updraft_dem`: DEM tile format, elevation lookup, download manifest format. _(needs: geo)_
- [ ] **agl-terrain** — AGL computation in core; terrain shading/hillshade on the map. _(needs: dem, frontend-map)_
- [ ] **map-inspector-terrain** — add terrain elevation, AGL information, and arrival height at the selected map position. _(needs: agl-terrain, final-glide, map-inspector-waypoints)_
- [ ] **glide-range** — terrain-aware glide range footprint ("reach polygon") rendered on the map. _(needs: agl-terrain, final-glide, compute-workers)_

## Airspace

- [ ] **geo-shapes** — cylinders, sectors, lines, arcs, polygons; point-inside tests and boundary-crossing detection. Shared by observation zones and airspace. _(needs: geo)_
- [x] **openair** — `libs/updraft_airspace` uses the `openair` crate to parse one source. It normalizes supported circles, arcs, and polygons into canonical polygon geometry. _(needs: geo)_
- [x] **airspace-dataset** — the core owns one canonical dataset and publishes its status and generation. The Tauri shell imports, persists, replaces, and removes the source. _(needs: openair, core-app)_
- [x] **airspace-on-map** — `updraft://localhost/airspace.geojson` projects the canonical dataset. MapLibre renders fills and outlines with class-based styles. _(needs: airspace-dataset, resource-scheme, frontend-map)_
- [ ] **airspace-filtering** — add altitude and class filters. Add per-zone enable and disable controls. _(needs: airspace-dataset, airspace-on-map)_
- [ ] **openaip-airspace** — OpenAIP airspace parser. _(needs: airspace-dataset)_
- [ ] **cub-airspace** — SeeYou CUB airspace parser. _(needs: airspace-dataset)_
- [ ] **sua-airspace** — SUA airspace parser. _(needs: airspace-dataset)_
- [ ] **airspace-warnings** — detect predicted incursions and publish graded warnings. Offer Until clear, 5 minute, 15 minute, 1 hour, and Today suppression. _(needs: airspace-dataset, geo-shapes, flight-modes, warning-presentation)_
- [x] **map-inspector-airspace** — show rendered airspaces at the selected map
  point and link to details from the current dataset. _(needs: airspace-on-map,
  map-inspector)_
- [ ] **obstacles** — obstacle databases and warnings. _(needs: airspace-warnings, dem)_

## Tasks

- [ ] **observation-zones** — OZ types (cylinder, FAI sector, keyhole, line) with entry/exit detection, per-point overrides. _(needs: geo-shapes)_
- [ ] **task-model** — task data model: task types, start/finish rules, validation, serde. _(needs: observation-zones, waypoint-db)_
- [ ] **task-engine** — in-flight progress: start detection/arming, automatic + manual turnpoint advance, and finish. Publish the current task point as the default Task target without stealing focus from another active target, and persist task state via snapshots for crash resume. _(needs: task-model, flight-modes, navigation-targets)_
- [ ] **task-manager-ui** — task build/edit UI (list editing + map rendering of the task). _(needs: task-model, frontend-map)_
- [ ] **map-inspector-task-points** — add task points and their task context to map-inspector results. _(needs: task-manager-ui, map-inspector-waypoints)_
- [ ] **task-calculator** — required speed, achieved speed, time gates, task arrival estimates, and task infobox values. _(needs: task-engine, final-glide)_
- [ ] **task-map-edit** — in-flight task editing and map-based point manipulation. _(needs: task-manager-ui, task-engine)_
- [ ] **aat** — assigned area tasks: isolines, target moving, min-time what-if range. _(needs: task-calculator)_
- [ ] **start-rules** — start gates, speed/height limits, PEV start. _(needs: task-engine)_
- [ ] **optimal-track** — wind-corrected required track indicator, relative-bearing integration, and AAT optimal arrow. _(needs: task-calculator, wind-circling)_
- [ ] **task-files** — task import/export file formats (including CUP task sections) + declaration data model (declaration to devices comes with device drivers). _(needs: task-model)_
- [ ] **fai-assistant** — FAI triangle rules + live triangle-closing guidance overlay. _(needs: task-engine, frontend-map)_
- [ ] **fai-badges** — badge/record rules and finish-below-start handling. _(needs: task-engine)_

## Traffic

- [x] **flarm-nmea** — FLARM sentences (`$PFLAA`/`$PFLAU`/`$PFLAC`, alarm levels) as an `updraft_nmea` slice. _(needs: nmea)_
- [x] **traffic-store** — the core converts relative PFLAA reports to positions. It stores target identity, aircraft type, altitude, track, and authoritative FLARM alarm level. It marks targets stale after 5 seconds and removes them after 30 seconds. _(needs: flarm-nmea, core-time, source-selection)_
- [x] **traffic-on-map** — MapLibre renders traffic symbols, authoritative alarm colours, altitude labels, track direction, and stale opacity. _(needs: traffic-store, frontend-map)_
- [ ] **traffic-trails** — render a short track trail for each moving target. _(needs: traffic-on-map)_
- [ ] **radar-view** — dedicated FLARM radar page (relative-position rose). _(needs: traffic-store)_
- [ ] **traffic-warnings** — present authoritative FLARM alarm levels through the shared warning UI. Support one-tap acknowledgement and reactivation when the device reports a new or worse alarm. Do not calculate collision risk in Updraft. _(needs: traffic-store, warning-presentation)_
- [ ] **traffic-lookup** — FlarmNet / OGN DDB parsing and ID→registration lookup, custom naming, buddy highlighting. _(needs: traffic-store)_
- [x] **map-inspector-traffic** — retain traffic at the selected map point,
  refresh those targets from topic updates, and link to live details. _(needs:
  traffic-on-map, map-inspector)_
- [ ] **traffic-list** — add a sortable list of all current traffic with search
  and direct detail navigation. _(needs: traffic-store, traffic-lookup)_
- [ ] **traffic-navigation-targets** — allow live traffic to occupy additional-target positions alongside waypoints and map positions, including focus and pinning, relative-altitude presentation, live guidance updates, and unavailable retention with report age and last-known-position guidance. _(needs: map-inspector-traffic, pinned-navigation-targets)_
- [ ] **ogn** — OGN traffic via the WeGlide Live API (bbox-scoped polling) + FLARM/OGN deduplication. _(needs: traffic-store, connectivity)_
- [ ] **adsb** — ADS-B In traffic: `libs/updraft_gdl90` (flag-delimited binary framing) as a second parser framing, plus PowerFLARM/Stratux wiring. _(needs: traffic-store)_

## Logging & recording

- [ ] **igc-write** — IGC recording: headers, B-records, pre-takeoff buffer, auto start/stop, interval control. Crash-safe: incremental flush-per-batch writes plus state snapshots so an interrupted flight resumes logging on restart. _(needs: igc-read, flight-modes)_
- [ ] **g-record** — tamper-evident G-record signing and validation. _(needs: igc-write)_
- [ ] **markers-pev** — manual/automatic markers and pilot events (1 Hz burst logging), markers on map, and the map-inspector **Drop marker** action. _(needs: igc-write, frontend-map, map-inspector-waypoints)_
- [ ] **replay-ui** — flight replay controls in the UI (file picker, speed, seek) on top of the replay engine. _(needs: replay, frontend-protocol)_
- [ ] **engine-monitoring** — ENL/MoP detection, engine hours, microphone-based ENL. _(needs: igc-write)_

## Map & UI polish

- [x] **map-follow** — follow fresh ownship positions until the user pans. Show a control that returns to the current position. Keep camera state while a settings route covers the map. _(needs: map-position, route-shell)_
- [ ] **map-orientation** — add track-up, north-up, and target-up modes. Add auto-zoom, circling zoom, and smart position offset. _(needs: map-follow, flight-modes)_
- [ ] **snail-trail** — flight trail with length modes and colouring by vario/altitude/speed. _(needs: frontend-map, vario-values)_
- [ ] **warning-presentation** — shared core warning identity, relevance, priority, and acknowledgement state. It provides fixed Situation Bar presentation with the highest-priority warning, body/details and `✓` actions, persistent pinned-target readouts with a temporary focused-target readout, warning-aware screen Map controls, global collision-warning overlays, and one-shot activation effects for native audio. _(needs: core-time, app-shell, pinned-navigation-targets)_
- [ ] **native-warning-notifications** — mirror each active warning in a platform notification while the app is in the background. Handle permissions, lifecycle, identity-based updates, removal, and tap-to-open routing. _(needs: warning-presentation, tauri-scaffold)_
- [ ] **infobox-pages** — linear infobox pages in a bottom portrait dock and side landscape dock, orientation-adaptive page swipes, a transient non-clickable page indicator, and automatic Thermal-page behavior. _(needs: infobox-values, flight-modes)_
- [ ] **infobox-layout-prototype** — compare ordered reflow, a normalized shared grid, and common dock geometry for mapping one saved infobox layout between portrait and landscape. _(needs: infobox-pages)_
- [x] **settings-persistence** — the core owns locale, unit, and external-device settings. The Tauri shell loads and atomically replaces one `settings.json` snapshot. _(needs: core-app, tauri-driver)_
- [x] **units-settings** — the settings UI selects altitude, distance, horizontal-speed, and vertical-speed units. Current flight and traffic values use these selections. _(needs: settings-persistence, frontend-protocol)_
- [ ] **configuration-profiles** — add named pilot, aircraft, and display profiles. Add explicit profile switching. _(needs: settings-persistence)_
- [ ] **infobox-layout-editor** — configured-page list including Thermal, page ordering and selection, a movable and resizable snap-grid editor with add/remove/duplicate/style actions, in-flight editing, and saved display-profile persistence. _(needs: infobox-layout-prototype, configuration-profiles)_
- [ ] **aircraft-profiles** — move the built-in catalogue into `updraft_aircraft_presets`. Create profiles from a preset or from scratch. Store field overrides, ballast, weights, registration, and competition ID. _(needs: configuration-profiles, glide-settings)_
- [ ] **startup-flow-prototype** — compare direct Flight View startup, a preflight dashboard, and a lightweight preflight overlay using aircraft and device readiness. An active flight always resumes directly. _(needs: app-shell, aircraft-profiles, device-manager)_
- [ ] **themes** — system/light/dark display setting persisted per display profile, explicit themes applied before first paint, and light-theme sunlight-readability targets validated outdoors. _(needs: frontend-scaffold, configuration-profiles)_
- [x] **i18n** — localization scaffolding (Paraglide JS) + German translation; land before untranslated strings accumulate. _(needs: frontend-scaffold)_
- [x] **about-page** — show the source repository, build commit, build time, and map data credits. _(needs: route-shell, i18n)_
- [ ] **disclaimer** — first-run "not a certified navigation source" dialog and about-screen text. _(needs: frontend-scaffold)_
- [ ] **webview-compat-warning** — detect webviews too old to render the MapLibre map and show an unsupported-version warning instead of a blank map. Repro: the Android emulator API 34 image ships WebView 113, which renders the map blank. API 35 (WebView 124) renders fine. _(needs: frontend-map)_
- [ ] **input-gestures** — configurable hardware buttons/keys and gesture bindings. _(needs: frontend-protocol)_
- [ ] **status-pages** — flight / times / system status dialogs. _(needs: infobox-values)_
- [ ] **sun-ephemeris** — `libs/updraft_sun`: sunrise/sunset/twilight math, time-of-day infobox values, and an "arrival past sunset" warning. _(needs: units, task-calculator)_
- [ ] **checklists** — user checklist files/pages. _(needs: frontend-protocol)_
- [ ] **weight-balance** — W&B / CG-envelope calculator. _(needs: aircraft-profiles)_
- [ ] **config-sharing** — configuration sharing via files / QR codes. _(needs: configuration-profiles)_
- [ ] **stopwatch-misc** — stopwatch, position/ATC report page. _(needs: infobox-values)_
- [ ] **ahrs-pfd** — attitude indicator / PFD from AHRS data. Add synthetic vision later. _(needs: lx-nmea, connection-ingestion)_

## Online services

Online services use async effect adapters. Bulk imagery and datasets use the resource path. They do not run as compute jobs.

- [ ] **connectivity** — online/offline detection and state in core, offline-first hooks (status indicator, queue-and-retry for uploads). _(needs: core-app)_
- [ ] **basemap-packs** — import one local MBTiles basemap and serve its vector tiles to MapLibre through `updraft://localhost/basemap/`. Evaluate other pack formats after the MVP. _(needs: resource-scheme, frontend-map)_
- [ ] **data-downloads** — in-app download manager for waypoint / airspace / map / DEM data with repository manifest and offline caching. _(needs: connectivity)_
- [ ] **metar-taf** — METAR/TAF fetch, decode, map flags, QNH extraction. _(needs: core-app, frontend-map)_
- [ ] **weather-overlays** — rain radar and satellite imagery overlays with time slider; forecast overlays (SkySight/TopMeteo) behind the same interface. _(needs: frontend-map)_
- [ ] **map-inspector-weather** — add weather features and their time/context information to map-inspector results. _(needs: weather-overlays, map-inspector-waypoints)_
- [ ] **wind-aloft** — multi-level forecast wind + live station wind display. _(needs: weather-overlays, wind-circling)_
- [ ] **notam** — NOTAM download rendered as airspace, filters, details. _(needs: airspace-dataset)_
- [ ] **task-download** — task download from SoaringSpot / WeGlide. _(needs: task-files)_
- [ ] **task-sharing** — task sharing via QR code / file share. _(needs: task-files)_
- [ ] **live-tracking** — position upload to OGN / SkyLines / LiveTrack24 style services. _(needs: flight-modes, connectivity)_
- [ ] **contest-upload** — one-tap post-flight upload (WeGlide, OLC, …). _(needs: igc-write, connectivity)_
- [ ] **thermal-hotspots** — crowd-sourced thermal hotspot overlays (kk7 / WeGlide). _(needs: frontend-map)_
- [ ] **charts** — approach charts / georeferenced chart overlays. _(needs: frontend-map, data-downloads)_
- [ ] **cloud-sync** — settings/task/waypoint sync via third-party cloud services. _(needs: configuration-profiles, connectivity)_
- [ ] **datalink-weather** — FIS-B / SiriusXM datalink weather. _(needs: weather-overlays, adsb)_

## Analysis & contest

- [ ] **barograph** — altitude trace page with working-band estimation. _(needs: igc-write, frontend-protocol)_
- [ ] **climb-stats** — per-climb history, thermal statistics, leg statistics pages. _(needs: thermal-assistant, task-engine)_
- [ ] **analysis-pages** — wind vs altitude, glide polar analysis, vario histogram, temperature trace. _(needs: barograph)_
- [ ] **cross-section** — airspace + terrain side-view profile ahead. _(needs: agl-terrain, airspace-dataset)_
- [ ] **contest-optimizer** — `libs/updraft_contest`: OLC/WeGlide/FAI rule sets, optimal path over the flown trace; designed for incremental re-optimization (retains state between rounds over the growing trace). _(needs: geo)_
- [ ] **live-scoring** — in-flight score and distance infoboxes plus the optimal path display. It is the first stateful user of the compute-worker path. _(needs: contest-optimizer, infobox-values, compute-workers)_
- [ ] **task-analysis** — post-flight per-leg statistics and AAT rendering. _(needs: task-engine, barograph)_

## Devices & platforms

- [ ] **serial-adapter** — serial/TTY adapter for desktop platforms with baud probing. _(needs: connection-ingestion)_
- [ ] **terminal-monitor** — terminal monitor page for I/O debugging. _(needs: connection-ingestion)_
- [ ] **devmode** — hidden developer mode (seven-tap unlock): byte-capture replay transport through the real parser, map rendering and data loading debug options. _(needs: frontend-protocol, connection-ingestion)_
- [x] **android-spp** — the mobile plugin connects to bonded Bluetooth Classic SPP devices. The Tauri shell owns retry and cancellation. Independent connections can run in parallel. _(needs: connection-ingestion, tauri-android, foreground-service)_
- [ ] **ble** — add Bluetooth Low Energy transport through a Tauri plugin. _(needs: connection-ingestion, tauri-scaffold)_
- [ ] **usb-otg** — USB-serial adapter via Android OTG. _(needs: serial-adapter, tauri-android)_
- [x] **internal-gnss** — the Android plugin supplies typed fixes from the device GNSS receiver. The source is the final GPS fallback after external devices. _(needs: core-time, tauri-android, foreground-service)_
- [ ] **additional-internal-sensors** — add pressure, acceleration, and rotation inputs on supported platforms. Add per-sensor configuration, permissions, and battery controls. _(needs: internal-gnss)_
- [x] **device-settings** — persist ordered TCP and Bluetooth SPP devices. The settings UI can add, edit, enable, disable, and delete them. Bluetooth creation uses bonded Android devices. _(needs: settings-persistence, tcp-client, android-spp)_
- [ ] **device-manager** — add user-controlled ordering, connection and source status, an Internal sensors row, capability chips, per-signal controls, NMEA pass-through, and output. _(needs: device-settings, gps-status, additional-internal-sensors)_
- [ ] **device-configs** — named device-config snapshots (device entries + priority order), aircraft-config linkage, manual save/load. _(needs: device-manager, aircraft-profiles)_
- [ ] **vendor-protocols** — driver/personality framework: sentence-family drivers, bidirectional settings sync with per-setting preferences, one-shot outbound operations, exclusive binary sessions. _(needs: device-manager)_
- [ ] **lxnav-sync** — LXNav personality: sync of MacCready, ballast, bugs, and QNH via `$PLXV*`. _(needs: vendor-protocols)_
- [ ] **flarm-declaration** — FLARM task declaration _(needs: vendor-protocols, task-files)_
- [ ] **lxnav-igc** — IGC file download from LXNav devices. _(needs: vendor-protocols)_
- [ ] **flarm-igc** — IGC file download from FLARM devices (FLARM, LX) via exclusive binary session. _(needs: vendor-protocols)_
- [x] **tauri-android** — Android build target: buildable debug APK, emulator smoke-test, single-ABI CI build. _(needs: tauri-scaffold)_
- [ ] **tauri-ios** — iOS build target. _(needs: tauri-scaffold)_
- [x] **keep-awake** — keep the Android screen awake while the activity is visible. _(needs: tauri-android)_
- [x] **foreground-service** — request location permission and keep the core and active transports running while the Android activity is absent. Hold a partial wake lock for the session. _(needs: tauri-android)_
- [ ] **mobile-emulator-tests** — automated Android emulator build/launch smoke-test in CI. _(needs: tauri-android, e2e-scaffold)_
- [ ] **sim-mode** — on-device simulator mode (fly without GPS): manual flying controls, direct position/altitude setting; activating sim/replay disables IGC logging and online data (weather, OGN). _(needs: replay)_
- [ ] **audio-alerts** — native audio plugin for one-shot airspace/traffic warning playback, driven directly from warning activation effects in the core and ready for future voice messages. It ships with the first release so airspace warnings are audible from day one. _(needs: warning-presentation, tauri-scaffold)_
- [ ] **battery-monitoring** — internal/external battery and voltage state. _(needs: device-manager)_
- [ ] **switch-inputs** — gear/flap warning digital inputs. _(needs: device-manager)_
- [ ] **radio** — radio frequency management via drivers. _(needs: vendor-protocols)_
- [ ] **xpdr** — transponder control via drivers. _(needs: vendor-protocols)_
- [ ] **two-seat** — front/rear cockpit sync of MC/ballast/target/wind. _(needs: secondary-clients, vendor-protocols)_
- [ ] **physiological** — heart rate / SpO₂ sensor input. _(needs: ble)_
- [ ] **audio-vario** — continuous audio vario via parameter-driven tone synthesis on the native audio thread (core streams climb rate). _(needs: audio-alerts, vario-values)_

## Future access and displays

- [ ] **http-api** — add an authenticated HTTP interface for commands, queries, topics, and resources only when a concrete remote-client feature needs it. Keep it outside the current app path. _(needs: core-app)_
- [ ] **secondary-clients** — add authenticated remote frontends, roles, permissions, and repeater-display behavior. _(needs: http-api, configuration-profiles)_
- [ ] **multi-display** — allow one Updraft instance to drive more than one independent display layout. Treat this as an optional far-future feature. _(needs: secondary-clients, configuration-profiles)_

## Distribution

- [ ] **releases** — packaging and release pipeline: GitHub Releases, Google Play, Apple App Store, F-Droid; platform-native update channels, no self-updater. Play Console foreground-service justification + demo video prepared before first submission. _(needs: foreground-service, tauri-ios, disclaimer)_

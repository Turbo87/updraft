# Implementation Roadmap

- **The core skeleton comes before any feature.** Every feature is "state + commands + computed values + a view", so the message protocol, the state struct, injected time, and one working transport are prerequisites for everything.
- **Replay is infrastructure, not a feature.** IGC parsing + a replay driver unlock the whole test strategy (e2e fixtures, regression tests, sim/demo mode), so they come right after the input pipeline exists.
- **Parser crates are hardened as they land.** Every parser crate carries proptest no-panic suites and snapshot tests against the shared `testdata/` corpus of recorded device captures (see [design/testing.md](design/testing.md)).
- **This is a rough plan, not a set of concrete tasks.** The exact shape of the individual steps is subject to change as the design evolves.

## Scaffolding

- [x] **workspace** — Cargo workspace with empty `core/` crate, rustfmt/clippy config, MIT/Apache-2.0 license files, CI workflow (fmt, clippy, test).
- [x] **frontend-scaffold** — SvelteKit + Svelte 5 + TypeScript skeleton in `frontend/`, Vitest component-test setup, lint/format config, CI job.
- [x] **server-scaffold** — `server/` axum crate: health endpoint, static serving of the frontend build, one integration test. _(needs: workspace)_

## Core skeleton

- [x] **units** — custom newtype quantities (length/altitude, speed, vertical speed, angle to start; pressure, mass, temperature added when features need them), conversions, and unit-system formatting. Start minimal and grow. _(needs: workspace)_
- [x] **geo** — lat/lon types, WGS84 distance/bearing/destination-point (via `geographiclib-rs`) with a haversine fast path, bounding boxes with antimeridian handling, `geo-types` interop behind a feature. Coordinate parsing/formatting is out of scope: each data-format crate parses its own wire format, display formatting is a UI concern. _(needs: units)_
- [ ] **core-state** — the central state struct, `Command`/`Query`/`Event` enums, `apply()` entry point, prioritized + coalescing input channel, serde serialization of the protocol. _(needs: units)_
- [ ] **core-time** — time as an input: clock/tick commands, deterministic timer queue, simulated-time test helpers, monotonic flight-time tracking. _(needs: core-state)_
- [ ] **core-subscriptions** — state-change notifications via named topics (last-value, keyed collection, events plus active set, reference) so UIs can subscribe to slices instead of full snapshots. _(needs: core-state)_
- [ ] **core-workers** — async computation pipeline: rate-limited stages, rayon worker pool posting results back as input messages, per-worker-kind staleness invalidation. _(needs: core-state, core-time)_

## Transports & walking skeleton

- [ ] **server-protocol** — axum: REST endpoints for queries/commands + state-change stream (WebSocket or SSE, whichever works best in practice), speaking the core protocol. _(needs: server-scaffold, core-subscriptions)_
- [ ] **server-auth** — session token required on all routes (commands, state stream, bulk data), `Host` allowlist, `Origin` validation on stream upgrades, strict CORS, password gate for non-loopback binding. _(needs: server-protocol)_
- [x] **server-shutdown** — graceful shutdown (Ctrl-C / SIGTERM) for the axum server. _(needs: server-scaffold)_
- [ ] **frontend-protocol** — TypeScript protocol types (generated from the Rust types, committed, with a CI drift check), state-stream client, Svelte store bridging core state into components. _(needs: frontend-scaffold, server-protocol)_
- [x] **frontend-map** — maplibre-gl map page with interim online basemap (OpenFreeMap, replaced by offline packs in basemap-packs), own-position symbol at a fixed placeholder position, manual pan/zoom. _(needs: frontend-scaffold)_
- [ ] **map-position** — own-position symbol driven by core state. Bulk geodata (tiles, overlays) is served as map sources by URL reference, never pushed through the message channel. _(needs: frontend-map, frontend-protocol)_
- [x] **tauri-scaffold** — Tauri shell (desktop first) hosting the frontend in the system webview; `pnpm tauri dev`/`build` loop and Linux CI build. _(needs: frontend-scaffold)_
- [ ] **tauri-protocol** — Tauri IPC bridge embedding the core in-process and exposing the same protocol as the server (commands/queries + state-change stream). _(needs: frontend-protocol, tauri-scaffold)_
- [ ] **bulk-data** — bulk geodata serving: native HTTP routes in the server and `updraft://` URI scheme in the Tauri shell, streaming tiles/GeoJSON as version-counted resources referenced by URL. _(needs: server-protocol, tauri-protocol)_
- [ ] **e2e-scaffold** — Playwright suite booting server + frontend, scripting position commands, asserting the map shows them. Establishes the CI rendering harness: software GL (SwiftShader/llvmpipe) for headless MapLibre and a `testMode` flag disabling map animation so tests await explicit "map idle" / "data version rendered" signals. Tests use a minimal inline map style instead of online tile services. This is the walking skeleton milestone. _(needs: map-position)_

## Sensor input & replay

- [ ] **nmea** — `libs/updraft_nmea`: sentence framing, checksum, GGA/RMC/GSA parsing into typed structs. _(needs: units, geo)_
- [ ] **nmea-airdata** — vendor sentences for baro altitude, IAS/TAS, TE vario (LXWP0, PGRMZ, POV, …). _(needs: nmea)_
- [ ] **io-adapters** — adapter trait for byte-stream devices, TCP client/server and UDP adapters, fake adapter for tests; framer + dispatcher routing each sentence to the parsers that claim it (multiple parsers per stream), with promiscuous identification mode, driver probe queries, and capability tagging; wire NMEA input into core position state. _(needs: nmea, core-time)_
- [ ] **gps-status** — fix quality, satellite info, positioning-source selection/fallback in state; status indicator in the UI. _(needs: io-adapters)_
- [ ] **igc-read** — `libs/updraft_igc`: parser for A/H/B/E/L records and extensions. _(needs: units, geo)_
- [ ] **replay** — replay engine feeding the core typed messages from IGC files at variable speed, bypassing the parser stack (byte-capture replay is a devmode tool); used for simulator mode, demo mode, and as the e2e fixture mechanism, migrating the e2e suite from scripted commands to replay fixtures. Input-log replay records external I/O results verbatim but recomputes pure worker results and injects them at their recorded position. _(needs: igc-read, core-time)_
- [ ] **input-recording** — opt-in recording of the core's input sequence to `captures/`, written incrementally like the IGC log; snapshot-seeded replay ("seed from snapshot X, replay from position N") alongside replay-from-empty. _(needs: replay)_
- [ ] **flight-modes** — takeoff/landing detection, cruise/circling detection, flight timer; mode exposed in state and shown in UI. _(needs: io-adapters)_
- [ ] **vario-values** — TE/netto/relative vario, integrator and thermal averagers computed in core from GPS + baro inputs. _(needs: nmea-airdata, flight-modes)_

## Glide computer

- [x] **polar** — glide polar model (quadratic coefficients, ballast/bugs degradation), a starter polar library, speed-to-fly and MacCready ring math. _(needs: units)_
- [ ] **glide-settings** — MacCready, ballast, bugs, safety heights / safety MC: commands, state, and a settings dialog. _(needs: polar, core-state, frontend-protocol)_
- [ ] **wind-circling** — wind estimation from circling drift; wind vector in state, manual override command, wind display. _(needs: flight-modes)_
- [ ] **wind-zigzag** — airspeed-based zigzag/EKF wind estimation, layered wind statistics, source blending. _(needs: wind-circling, nmea-airdata)_
- [ ] **final-glide** — wind-corrected arrival altitude for an arbitrary target (Mc and Mc-0), safety-height aware. _(needs: glide-settings, wind-circling)_
- [ ] **speed-to-fly** — STF / speed command values, dolphin speed, auto MacCready modes. _(needs: glide-settings, vario-values)_
- [ ] **datafields-v1** — configurable data-field grid (fixed geometry, selectable values, tap-to-edit MC); the first set of altitude / speed / direction / time values. _(needs: frontend-protocol)_
- [ ] **thermal-assistant** — climb sampling around the circle, centering aid view, thermal profile (climb vs altitude band). _(needs: vario-values)_
- [ ] **thermal-history** — own-climb thermal markers on the map with wind drift compensation. _(needs: thermal-assistant, wind-circling, frontend-map)_
- [ ] **density-altitude** — pressure/density-altitude tools, potential-temperature trigger aid. _(needs: nmea-airdata)_

## Waypoints & navigation

- [ ] **cup** — `libs/updraft_cup`: SeeYou CUP waypoint/task file parser (CUPX and other formats come later). _(needs: units, geo)_
- [ ] **waypoint-db** — core waypoint store: multiple files, landable distinction, search, nearest-N queries. _(needs: cup, core-state)_
- [ ] **file-import** — file import via OS file picker and share intent, routed by file type to the matching store. _(needs: waypoint-db, tauri-protocol)_
- [ ] **cupx** — SeeYou CUPX waypoint files (CUP plus embedded images). _(needs: cup)_
- [ ] **openaip-waypoints** — OpenAIP airport/waypoint parser. _(needs: waypoint-db)_
- [ ] **gpx-waypoints** — GPX waypoint parser. _(needs: waypoint-db)_
- [ ] **geojson-waypoints** — GeoJSON waypoint parser. _(needs: waypoint-db)_
- [ ] **dat-waypoints** — Cambridge DAT waypoint parser. _(needs: waypoint-db)_
- [ ] **wpt-waypoints** — Winpilot/CompeGPS WPT waypoint parser. _(needs: waypoint-db)_
- [ ] **waypoints-on-map** — waypoint/landable symbology, labels, and zoom-dependent declutter. _(needs: waypoint-db, frontend-map)_
- [ ] **goto** — direct-to navigation: active target, bearing/distance/ETA values, course line on the map. _(needs: waypoint-db, datafields-v1)_
- [ ] **waypoint-details** — details dialog (elevation, runway, frequency, notes) and "what's here" multi-object map query. _(needs: waypoints-on-map)_
- [ ] **arrival-heights** — reachability of landables via final glide; arrival-height labels and reachability colouring. _(needs: final-glide, waypoints-on-map)_
- [ ] **alternates** — best-alternate selection, alternates dialog, abort mode. _(needs: arrival-heights, goto)_
- [ ] **nearest-lists** — sortable nearest waypoint/landable/airfield list pages. _(needs: arrival-heights)_
- [ ] **ga-routes** — GA flight-route editor (leg-based, distinct from scored tasks). _(needs: waypoint-db, frontend-map)_
- [ ] **vnav** — VNAV to altitude constraints. _(needs: final-glide, goto)_

## Terrain

- [ ] **dem** — `libs/updraft_dem`: DEM tile format, elevation lookup, download manifest format. _(needs: geo)_
- [ ] **agl-terrain** — AGL computation in core; terrain shading/hillshade on the map. _(needs: dem, frontend-map)_
- [ ] **glide-range** — terrain-aware glide range footprint ("reach polygon") rendered on the map. _(needs: agl-terrain, final-glide, core-workers)_

## Airspace

- [ ] **geo-shapes** — cylinders, sectors, lines, arcs, polygons; point-inside tests and boundary-crossing detection. Shared by observation zones and airspace. _(needs: geo)_
- [ ] **openair** — `libs/updraft_openair`: OpenAir airspace file parser. _(needs: geo-shapes)_
- [ ] **airspace-store** — core airspace state: classes, altitude/class filters, per-zone enable/disable. _(needs: openair, core-state)_
- [ ] **openaip-airspace** — OpenAIP airspace parser. _(needs: airspace-store)_
- [ ] **cub-airspace** — SeeYou CUB airspace parser. _(needs: airspace-store)_
- [ ] **sua-airspace** — SUA airspace parser. _(needs: airspace-store)_
- [ ] **airspace-on-map** — airspace rendering with per-class styling and altitude filtering. _(needs: airspace-store, frontend-map)_
- [ ] **airspace-warnings** — predicted incursion detection, graded warnings, acknowledge/dismiss with duration. _(needs: airspace-store, flight-modes)_
- [ ] **airspace-details** — vicinity list, details dialog, "what's here" integration. _(needs: airspace-on-map, waypoint-details)_
- [ ] **obstacles** — obstacle databases and warnings. _(needs: airspace-warnings, dem)_

## Tasks

- [ ] **observation-zones** — OZ types (cylinder, FAI sector, keyhole, line) with entry/exit detection, per-point overrides. _(needs: geo-shapes)_
- [ ] **task-model** — task data model: task types, start/finish rules, validation, serde. _(needs: observation-zones, waypoint-db)_
- [ ] **task-engine** — in-flight progress: start detection/arming, automatic + manual turnpoint advance, finish; task state in core, persisted via state snapshots for crash resume. _(needs: task-model, flight-modes)_
- [ ] **task-manager-ui** — task build/edit UI (list editing + map rendering of the task). _(needs: task-model, frontend-map)_
- [ ] **task-calculator** — required speed, achieved speed, time gates, task arrival estimates; task data fields. _(needs: task-engine, final-glide)_
- [ ] **task-map-edit** — in-flight task editing and map-based point manipulation. _(needs: task-manager-ui, task-engine)_
- [ ] **aat** — assigned area tasks: isolines, target moving, min-time what-if range. _(needs: task-calculator)_
- [ ] **start-rules** — start gates, speed/height limits, PEV start. _(needs: task-engine)_
- [ ] **optimal-track** — optimal cruise track indicator, AAT optimal arrow. _(needs: task-calculator, wind-circling)_
- [ ] **task-files** — task import/export file formats (including CUP task sections) + declaration data model (declaration to devices comes with device drivers). _(needs: task-model)_
- [ ] **fai-assistant** — FAI triangle rules + live triangle-closing guidance overlay. _(needs: task-engine, frontend-map)_
- [ ] **fai-badges** — badge/record rules and finish-below-start handling. _(needs: task-engine)_

## Traffic

- [ ] **flarm** — `libs/updraft_flarm`: PFLAA/PFLAU parsing, alarm levels, FLARM configuration sentences. _(needs: nmea)_
- [ ] **traffic-store** — traffic targets in core: aging, threat levels, relative geometry. _(needs: flarm, core-time)_
- [ ] **traffic-on-map** — traffic symbols, threat colouring, labels, short track trails. _(needs: traffic-store, frontend-map)_
- [ ] **radar-view** — dedicated FLARM radar page (relative-position rose). _(needs: traffic-store)_
- [ ] **traffic-warnings** — collision warning UI with alarm levels and acknowledgement; hook for audio alerts. _(needs: traffic-store)_
- [ ] **traffic-lookup** — FlarmNet / OGN DDB parsing and ID→registration lookup, custom naming, buddy highlighting. _(needs: traffic-store)_
- [ ] **traffic-details** — per-target details dialog and sortable traffic list. _(needs: traffic-on-map, traffic-lookup)_
- [ ] **ogn** — OGN traffic via the WeGlide Live API (bbox-scoped polling) + FLARM/OGN deduplication. _(needs: traffic-store, connectivity)_
- [ ] **adsb** — ADS-B In traffic (GDL90 parsing, PowerFLARM/Stratux). _(needs: traffic-store)_

## Logging & recording

- [ ] **igc-write** — IGC recording: headers, B-records, pre-takeoff buffer, auto start/stop, interval control. Crash-safe: incremental flush-per-batch writes plus state snapshots so an interrupted flight resumes logging on restart. _(needs: igc-read, flight-modes)_
- [ ] **g-record** — tamper-evident G-record signing and validation. _(needs: igc-write)_
- [ ] **markers-pev** — manual/automatic markers and pilot events (1 Hz burst logging), markers on map. _(needs: igc-write, frontend-map)_
- [ ] **replay-ui** — flight replay controls in the UI (file picker, speed, seek) on top of the replay engine. _(needs: replay, frontend-protocol)_
- [ ] **engine-monitoring** — ENL/MoP detection, engine hours, microphone-based ENL. _(needs: igc-write)_

## Map & UI polish

- [ ] **map-orientation** — track-up / north-up / target-up, auto-zoom, circling zoom, smart offset position. _(needs: frontend-map, flight-modes)_
- [ ] **snail-trail** — flight trail with length modes and colouring by vario/altitude/speed. _(needs: frontend-map, vario-values)_
- [ ] **datafield-pages** — multiple data-field pages/layouts, per-flight-mode auto switching, bottom nav-box bar. _(needs: datafields-v1, flight-modes)_
- [ ] **units-settings** — per-quantity unit configuration UI wired through all displayed values. _(needs: datafields-v1)_
- [ ] **settings-persistence** — configuration profiles (per pilot/per aircraft), settings persistence adapter, profile switching. _(needs: core-state)_
- [ ] **aircraft-profiles** — move the built-in catalogue out of the polar store into `updraft_aircraft_presets` (aircraft presets); aircraft profiles created from a preset or from scratch, with per-field overrides, ballast/weights, and registration/comp ID. _(needs: settings-persistence, glide-settings)_
- [ ] **themes** — day/night/high-contrast modes, sunlight-readability contrast targets validated outdoors, auto-brightness hooks. _(needs: frontend-scaffold)_
- [x] **i18n** — localization scaffolding (Paraglide JS) + German translation; land before untranslated strings accumulate. _(needs: frontend-scaffold)_
- [ ] **disclaimer** — first-run "not a certified navigation source" dialog and about-screen text. _(needs: frontend-scaffold)_
- [ ] **input-gestures** — configurable hardware buttons/keys and gesture bindings. _(needs: frontend-protocol)_
- [ ] **status-pages** — flight / times / system status dialogs. _(needs: datafields-v1)_
- [ ] **sun-ephemeris** — `libs/updraft_sun`: sunrise/sunset/twilight math; time-of-day data fields and "arrival past sunset" warning. _(needs: units, task-calculator)_
- [ ] **checklists** — user checklist files/pages. _(needs: frontend-protocol)_
- [ ] **weight-balance** — W&B / CG-envelope calculator. _(needs: aircraft-profiles)_
- [ ] **config-sharing** — configuration sharing via files / QR codes. _(needs: settings-persistence)_
- [ ] **stopwatch-misc** — stopwatch, position/ATC report page. _(needs: datafields-v1)_
- [ ] **ahrs-pfd** — attitude indicator / PFD from AHRS data; synthetic vision later. _(needs: nmea-airdata, io-adapters)_

## Online services

- [ ] **connectivity** — online/offline detection and state in core, offline-first hooks (status indicator, queue-and-retry for uploads). _(needs: core-state)_
- [ ] **basemap-packs** — offline basemap packs (PMTiles or MBTiles, format TBD) stored on device, served to MapLibre through the bulk geodata path. _(needs: bulk-data, frontend-map)_
- [ ] **data-downloads** — in-app download manager for waypoint / airspace / map / DEM data with repository manifest and offline caching. _(needs: connectivity)_
- [ ] **metar-taf** — METAR/TAF fetch, decode, map flags, QNH extraction. _(needs: core-state, frontend-map)_
- [ ] **weather-overlays** — rain radar and satellite imagery overlays with time slider; forecast overlays (SkySight/TopMeteo) behind the same interface. _(needs: frontend-map)_
- [ ] **wind-aloft** — multi-level forecast wind + live station wind display. _(needs: weather-overlays, wind-circling)_
- [ ] **notam** — NOTAM download rendered as airspace, filters, details. _(needs: airspace-store)_
- [ ] **task-download** — task download from SoaringSpot / WeGlide. _(needs: task-files)_
- [ ] **task-sharing** — task sharing via QR code / file share. _(needs: task-files)_
- [ ] **live-tracking** — position upload to OGN / SkyLines / LiveTrack24 style services. _(needs: flight-modes, connectivity)_
- [ ] **contest-upload** — one-tap post-flight upload (WeGlide, OLC, …). _(needs: igc-write, connectivity)_
- [ ] **thermal-hotspots** — crowd-sourced thermal hotspot overlays (kk7 / WeGlide). _(needs: frontend-map)_
- [ ] **charts** — approach charts / georeferenced chart overlays. _(needs: frontend-map, data-downloads)_
- [ ] **cloud-sync** — settings/task/waypoint sync via third-party cloud services. _(needs: settings-persistence, connectivity)_
- [ ] **datalink-weather** — FIS-B / SiriusXM datalink weather. _(needs: weather-overlays, adsb)_

## Analysis & contest

- [ ] **barograph** — altitude trace page with working-band estimation. _(needs: igc-write, frontend-protocol)_
- [ ] **climb-stats** — per-climb history, thermal statistics, leg statistics pages. _(needs: thermal-assistant, task-engine)_
- [ ] **analysis-pages** — wind vs altitude, glide polar analysis, vario histogram, temperature trace. _(needs: barograph)_
- [ ] **cross-section** — airspace + terrain side-view profile ahead. _(needs: agl-terrain, airspace-store)_
- [ ] **contest-optimizer** — `libs/updraft_contest`: OLC/WeGlide/FAI rule sets, optimal path over the flown trace. _(needs: geo)_
- [ ] **live-scoring** — in-flight optimization: score/achieved-distance data fields, optimal path display. _(needs: contest-optimizer, datafields-v1)_
- [ ] **task-analysis** — post-flight per-leg statistics and AAT rendering. _(needs: task-engine, barograph)_

## Devices & platforms

- [ ] **serial-adapter** — serial/TTY adapter for desktop platforms with baud probing. _(needs: io-adapters)_
- [ ] **terminal-monitor** — terminal monitor page for I/O debugging. _(needs: io-adapters)_
- [ ] **devmode** — hidden developer mode (seven-tap unlock): byte-capture replay transport through the real parser stack, map rendering and data loading debug options. _(needs: frontend-protocol, io-adapters)_
- [ ] **bluetooth** — Bluetooth SPP adapter via Tauri plugin (per-platform permissions). _(needs: io-adapters, tauri-protocol)_
- [ ] **ble** — Bluetooth BLE adapter via Tauri plugin (per-platform permissions). _(needs: io-adapters, tauri-protocol)_
- [ ] **usb-otg** — USB-serial adapter via Android OTG. _(needs: serial-adapter, tauri-android)_
- [ ] **internal-sensors** — internal GPS and pressure sensor input via Tauri plugins, injected as typed messages, ranked below external devices; always-on by default (WeGlide-valid IGC logs) with a battery-saver setting. _(needs: core-time, tauri-protocol)_
- [ ] **device-manager** — devices screen (user-ordered priority list), multi-device value merging, priority/fallback, NMEA pass-through/output. _(needs: io-adapters, gps-status)_
- [ ] **device-configs** — named device-config snapshots (device entries + priority order), aircraft-config linkage, manual save/load. _(needs: device-manager, aircraft-profiles)_
- [ ] **vendor-protocols** — driver/personality framework: sentence-family drivers, bidirectional settings sync with per-setting preferences, one-shot outbound operations, exclusive binary sessions. _(needs: device-manager)_
- [ ] **lxnav-sync** — LXNav personality: sync of MacCready, ballast, bugs, and QNH via `$PLXV*`. _(needs: vendor-protocols)_
- [ ] **flarm-declaration** — FLARM task declaration _(needs: vendor-protocols, task-files)_
- [ ] **lxnav-igc** — IGC file download from LXNav devices. _(needs: vendor-protocols)_
- [ ] **flarm-igc** — IGC file download from FLARM devices (FLARM, LX) via exclusive binary session. _(needs: vendor-protocols)_
- [ ] **tauri-android** — Android build target: buildable debug APK, emulator smoke-test, single-ABI CI build. _(needs: tauri-scaffold)_
- [ ] **tauri-ios** — iOS build target. _(needs: tauri-scaffold)_
- [ ] **keep-awake** — screen keep-awake while flying. _(needs: tauri-scaffold)_
- [ ] **foreground-service** — location permission and background execution via an Android foreground service keeping the core alive off-screen. _(needs: tauri-android, internal-sensors)_
- [ ] **mobile-emulator-tests** — automated Android emulator build/launch smoke-test in CI. _(needs: tauri-android, e2e-scaffold)_
- [ ] **sim-mode** — on-device simulator mode (fly without GPS): manual flying controls, direct position/altitude setting; activating sim/replay disables IGC logging and online data (weather, OGN). _(needs: replay)_
- [ ] **secondary-clients** — primary/secondary operation: auth, roles & permissions for remote frontends, repeater display mode. _(needs: server-protocol, settings-persistence)_
- [ ] **audio-alerts** — native audio plugin for airspace/traffic warning playback, driven directly from the core; ships with the first release so airspace warnings are audible from day one. _(needs: airspace-warnings, tauri-protocol)_
- [ ] **battery-monitoring** — internal/external battery and voltage state. _(needs: device-manager)_
- [ ] **switch-inputs** — gear/flap warning digital inputs. _(needs: device-manager)_
- [ ] **radio** — radio frequency management via drivers. _(needs: vendor-protocols)_
- [ ] **xpdr** — transponder control via drivers. _(needs: vendor-protocols)_
- [ ] **two-seat** — front/rear cockpit sync of MC/ballast/target/wind. _(needs: secondary-clients, vendor-protocols)_
- [ ] **physiological** — heart rate / SpO₂ sensor input. _(needs: ble)_
- [ ] **audio-vario** — continuous audio vario via parameter-driven tone synthesis on the native audio thread (core streams climb rate). _(needs: audio-alerts, vario-values)_

## Distribution

- [ ] **releases** — packaging and release pipeline: GitHub Releases, Google Play, Apple App Store, F-Droid; platform-native update channels, no self-updater. Play Console foreground-service justification + demo video prepared before first submission. _(needs: foreground-service, tauri-ios, disclaimer)_

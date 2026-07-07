# Implementation Roadmap

- **The core skeleton comes before any feature.** Every feature is "state +
  commands + computed values + a view", so the message protocol, the state
  struct, injected time, and one working transport are prerequisites for
  everything.
- **Replay is infrastructure, not a feature.** IGC parsing + a replay driver
  unlock the whole test strategy (e2e fixtures, regression tests, sim/demo
  mode), so they come right after the input pipeline exists.
- **Parser crates are hardened as they land.** Every parser crate carries
  proptest no-panic suites and snapshot tests against the shared `testdata/`
  corpus of recorded device captures (see
  [design/testing.md](design/testing.md)).
- **This is a rough plan, not a set of concrete tasks.** The exact shape of the
  individual steps is subject to change as the design evolves.

## Scaffolding

- [x] **workspace** — Cargo workspace with empty `core/` crate,
      rustfmt/clippy config, MIT/Apache-2.0 license files, CI workflow
      (fmt, clippy, test).
- [x] **frontend-scaffold** — SvelteKit + Svelte 5 + TypeScript skeleton in
      `frontend/`, Vitest component-test setup, lint/format config, CI job.
- [x] **server-scaffold** — `server/` axum crate: health endpoint, static
      serving of the frontend build, one integration test. *(needs:
      workspace)*

## Core skeleton

- [x] **units** — custom newtype quantities (length/altitude, speed, vertical
      speed, angle to start; pressure, mass, temperature added when features
      need them), conversions, and unit-system formatting. Start minimal and
      grow. *(needs: workspace)*
- [ ] **geo** — lat/lon types, WGS84 distance/bearing/destination-point,
      bounding boxes, coordinate parsing/formatting. *(needs: units)*
- [ ] **core-state** — the central state struct, `Command`/`Query`/`Event`
      enums, `apply()` entry point, prioritized + coalescing input channel,
      serde serialization of the protocol. *(needs: units)*
- [ ] **core-time** — time as an input: clock/tick commands, deterministic
      timer queue, simulated-time test helpers, monotonic flight-time
      tracking. *(needs: core-state)*
- [ ] **core-subscriptions** — state-change notifications via named topics
      (last-value, keyed collection, events plus active set, reference) so
      UIs can subscribe to slices instead of full snapshots. *(needs:
      core-state)*
- [ ] **core-workers** — async computation pipeline: rate-limited stages,
      rayon worker pool posting results back as input messages,
      per-worker-kind staleness invalidation. *(needs: core-state,
      core-time)*

## Transports & walking skeleton

- [ ] **server-protocol** — axum: REST endpoints for queries/commands +
      state-change stream (WebSocket or SSE, whichever works best in
      practice), speaking the core protocol.
      *(needs: server-scaffold, core-subscriptions)*
- [ ] **server-auth** — session token required on all routes (commands,
      state stream, bulk data), `Host` allowlist, `Origin` validation on
      stream upgrades, strict CORS, password gate for non-loopback binding.
      *(needs: server-protocol)*
- [x] **server-shutdown** — graceful shutdown (Ctrl-C / SIGTERM) for the
      axum server. *(needs: server-scaffold)*
- [ ] **frontend-protocol** — TypeScript protocol types (generated from the
      Rust types, committed, with a CI drift check), state-stream client, Svelte
      store bridging core state into components.
      *(needs: frontend-scaffold, server-protocol)*
- [ ] **frontend-map** — maplibre-gl map page with basemap style and an
      own-position symbol driven by core state; manual pan/zoom. Bulk geodata
      (tiles, overlays) is served as map sources by URL reference, never
      pushed through the message channel. *(needs: frontend-protocol)*
- [ ] **tauri-scaffold** — Tauri shell (desktop first) embedding the core
      in-process, IPC bridge exposing the same protocol as the server.
      *(needs: frontend-protocol)*
- [ ] **bulk-data** — bulk geodata serving: native HTTP routes in the
      server and `updraft://` URI scheme in the Tauri shell, streaming
      tiles/GeoJSON as version-counted resources referenced by URL.
      *(needs: server-protocol, tauri-scaffold)*
- [ ] **e2e-scaffold** — Playwright suite booting server + frontend,
      scripting position commands, asserting the map shows them. Establishes
      the CI rendering harness: software GL (SwiftShader/llvmpipe) for
      headless MapLibre and a `testMode` flag disabling map animation so
      tests await explicit "map idle" / "data version rendered" signals. This
      is the walking skeleton milestone. *(needs: frontend-map)*

## Sensor input & replay

- [ ] **nmea** — `libs/updraft_nmea`: sentence framing, checksum,
      GGA/RMC/GSA parsing into typed structs. *(needs: units, geo)*
- [ ] **nmea-airdata** — vendor sentences for baro altitude, IAS/TAS, TE
      vario (LXWP0, PGRMZ, POV, …). *(needs: nmea)*
- [ ] **io-adapters** — adapter trait for byte-stream devices, TCP
      client/server and UDP adapters, fake adapter for tests; framer +
      dispatcher routing each sentence to the parsers that claim it
      (multiple parsers per stream), with promiscuous identification mode,
      driver probe queries, and capability tagging; wire NMEA input into
      core position state. *(needs: nmea, core-time)*
- [ ] **gps-status** — fix quality, satellite info, positioning-source
      selection/fallback in state; status indicator in the UI.
      *(needs: io-adapters)*
- [ ] **igc-read** — `libs/updraft_igc`: parser for A/H/B/E/L records and
      extensions. *(needs: units, geo)*
- [ ] **replay** — replay engine feeding the core typed messages from IGC
      files at variable speed, bypassing the parser stack (byte-capture
      replay is a devmode tool); used for simulator mode, demo mode, and as
      the e2e fixture mechanism, migrating the e2e suite from scripted
      commands to replay fixtures. Input-log replay records external I/O
      results verbatim but recomputes pure worker results and injects them
      at their recorded position. *(needs: igc-read, core-time)*
- [ ] **input-recording** — opt-in recording of the core's input sequence
      to `captures/`, written incrementally like the IGC log;
      snapshot-seeded replay ("seed from snapshot X, replay from position
      N") alongside replay-from-empty. *(needs: replay)*
- [ ] **flight-modes** — takeoff/landing detection, cruise/circling
      detection, flight timer; mode exposed in state and shown in UI.
      *(needs: io-adapters)*
- [ ] **vario-values** — TE/netto/relative vario, integrator and thermal
      averagers computed in core from GPS + baro inputs.
      *(needs: nmea-airdata, flight-modes)*

## Glide computer

- [ ] **polar** — glide polar model (quadratic coefficients, ballast/bugs
      degradation), a starter polar library, speed-to-fly and MacCready
      ring math. *(needs: units)*
- [ ] **glide-settings** — MacCready, ballast, bugs, safety heights /
      safety MC: commands, state, and a settings dialog. *(needs: polar,
      core-state, frontend-protocol)*
- [ ] **wind-circling** — wind estimation from circling drift; wind vector
      in state, manual override command, wind display. *(needs:
      flight-modes)*
- [ ] **wind-zigzag** — airspeed-based zigzag/EKF wind estimation, layered
      wind statistics, source blending. *(needs: wind-circling,
      nmea-airdata)*
- [ ] **final-glide** — wind-corrected arrival altitude for an arbitrary
      target (Mc and Mc-0), safety-height aware. *(needs: glide-settings,
      wind-circling)*
- [ ] **speed-to-fly** — STF / speed command values, dolphin speed, auto
      MacCready modes. *(needs: glide-settings, vario-values)*
- [ ] **datafields-v1** — configurable data-field grid (fixed geometry,
      selectable values, tap-to-edit MC); the first set of altitude / speed
      / direction / time values. *(needs: frontend-protocol)*
- [ ] **thermal-assistant** — climb sampling around the circle, centering
      aid view, thermal profile (climb vs altitude band). *(needs:
      vario-values)*
- [ ] **thermal-history** — own-climb thermal markers on the map with wind
      drift compensation. *(needs: thermal-assistant, wind-circling,
      frontend-map)*
- [ ] **density-altitude** — pressure/density-altitude tools,
      potential-temperature trigger aid. *(needs: nmea-airdata)*

## Waypoints & navigation

- [ ] **cup** — `libs/updraft_cup`: SeeYou CUP waypoint/task file parser
      (CUPX and other formats come later). *(needs: units, geo)*
- [ ] **waypoint-db** — core waypoint store: multiple files, landable
      distinction, search, nearest-N queries. *(needs: cup, core-state)*
- [ ] **file-import** — file import via OS file picker and share intent,
      routed by file type to the matching store. *(needs: waypoint-db,
      tauri-scaffold)*
- [ ] **cupx** — SeeYou CUPX waypoint files (CUP plus embedded images).
      *(needs: cup)*
- [ ] **openaip-waypoints** — OpenAIP airport/waypoint parser. *(needs:
      waypoint-db)*
- [ ] **gpx-waypoints** — GPX waypoint parser. *(needs: waypoint-db)*
- [ ] **geojson-waypoints** — GeoJSON waypoint parser. *(needs:
      waypoint-db)*
- [ ] **dat-waypoints** — Cambridge DAT waypoint parser. *(needs:
      waypoint-db)*
- [ ] **wpt-waypoints** — Winpilot/CompeGPS WPT waypoint parser. *(needs:
      waypoint-db)*
- [ ] **waypoints-on-map** — waypoint/landable symbology, labels, and
      zoom-dependent declutter. *(needs: waypoint-db, frontend-map)*
- [ ] **goto** — direct-to navigation: active target, bearing/distance/ETA
      values, course line on the map. *(needs: waypoint-db, datafields-v1)*
- [ ] **waypoint-details** — details dialog (elevation, runway, frequency,
      notes) and "what's here" multi-object map query. *(needs:
      waypoints-on-map)*
- [ ] **arrival-heights** — reachability of landables via final glide;
      arrival-height labels and reachability colouring. *(needs:
      final-glide, waypoints-on-map)*
- [ ] **alternates** — best-alternate selection, alternates dialog, abort
      mode. *(needs: arrival-heights, goto)*
- [ ] **nearest-lists** — sortable nearest waypoint/landable/airfield list
      pages. *(needs: arrival-heights)*
- [ ] **ga-routes** — GA flight-route editor (leg-based, distinct from
      scored tasks). *(needs: waypoint-db, frontend-map)*
- [ ] **vnav** — VNAV to altitude constraints. *(needs: final-glide,
      goto)*

## Terrain

- [ ] **dem** — `libs/updraft_dem`: DEM tile format, elevation lookup,
      download manifest format. *(needs: geo)*
- [ ] **agl-terrain** — AGL computation in core; terrain shading/hillshade
      on the map. *(needs: dem, frontend-map)*
- [ ] **glide-range** — terrain-aware glide range footprint ("reach
      polygon") rendered on the map. *(needs: agl-terrain, final-glide,
      core-workers)*

## Airspace

- [ ] **geo-shapes** — cylinders, sectors, lines, arcs, polygons;
      point-inside tests and boundary-crossing detection. Shared by
      observation zones and airspace. *(needs: geo)*
- [ ] **openair** — `libs/updraft_openair`: OpenAir airspace file parser.
      *(needs: geo-shapes)*
- [ ] **airspace-store** — core airspace state: classes, altitude/class
      filters, per-zone enable/disable. *(needs: openair, core-state)*
- [ ] **openaip-airspace** — OpenAIP airspace parser. *(needs:
      airspace-store)*
- [ ] **cub-airspace** — SeeYou CUB airspace parser. *(needs:
      airspace-store)*
- [ ] **sua-airspace** — SUA airspace parser. *(needs: airspace-store)*
- [ ] **airspace-on-map** — airspace rendering with per-class styling and
      altitude filtering. *(needs: airspace-store, frontend-map)*
- [ ] **airspace-warnings** — predicted incursion detection, graded
      warnings, acknowledge/dismiss with duration. *(needs: airspace-store,
      flight-modes)*
- [ ] **airspace-details** — vicinity list, details dialog, "what's here"
      integration. *(needs: airspace-on-map, waypoint-details)*
- [ ] **obstacles** — obstacle databases and warnings. *(needs:
      airspace-warnings, dem)*

## Tasks

- [ ] **observation-zones** — OZ types (cylinder, FAI sector, keyhole,
      line) with entry/exit detection, per-point overrides. *(needs:
      geo-shapes)*
- [ ] **task-model** — task data model: task types, start/finish rules,
      validation, serde. *(needs: observation-zones, waypoint-db)*
- [ ] **task-engine** — in-flight progress: start detection/arming,
      automatic + manual turnpoint advance, finish; task state in core,
      persisted via state snapshots for crash resume.
      *(needs: task-model, flight-modes)*
- [ ] **task-manager-ui** — task build/edit UI (list editing + map
      rendering of the task). *(needs: task-model, frontend-map)*
- [ ] **task-calculator** — required speed, achieved speed, time gates,
      task arrival estimates; task data fields. *(needs: task-engine,
      final-glide)*
- [ ] **task-map-edit** — in-flight task editing and map-based point
      manipulation. *(needs: task-manager-ui, task-engine)*
- [ ] **aat** — assigned area tasks: isolines, target moving, min-time
      what-if range. *(needs: task-calculator)*
- [ ] **start-rules** — start gates, speed/height limits, PEV start.
      *(needs: task-engine)*
- [ ] **optimal-track** — optimal cruise track indicator, AAT optimal
      arrow. *(needs: task-calculator, wind-circling)*
- [ ] **task-files** — task import/export file formats (including CUP task
      sections) + declaration data model (declaration to devices comes with
      device drivers). *(needs: task-model)*
- [ ] **fai-assistant** — FAI triangle rules + live triangle-closing
      guidance overlay. *(needs: task-engine, frontend-map)*
- [ ] **fai-badges** — badge/record rules and finish-below-start handling.
      *(needs: task-engine)*

## Traffic

- [ ] **flarm** — `libs/updraft_flarm`: PFLAA/PFLAU parsing, alarm levels,
      FLARM configuration sentences. *(needs: nmea)*
- [ ] **traffic-store** — traffic targets in core: aging, threat levels,
      relative geometry. *(needs: flarm, core-time)*
- [ ] **traffic-on-map** — traffic symbols, threat colouring, labels,
      short track trails. *(needs: traffic-store, frontend-map)*
- [ ] **radar-view** — dedicated FLARM radar page (relative-position
      rose). *(needs: traffic-store)*
- [ ] **traffic-warnings** — collision warning UI with alarm levels and
      acknowledgement; hook for audio alerts. *(needs: traffic-store)*
- [ ] **traffic-lookup** — FlarmNet / OGN DDB parsing and ID→registration
      lookup, custom naming, buddy highlighting. *(needs: traffic-store)*
- [ ] **traffic-details** — per-target details dialog and sortable traffic
      list. *(needs: traffic-on-map, traffic-lookup)*
- [ ] **ogn** — OGN traffic via the WeGlide Live API (bbox-scoped polling)
      + FLARM/OGN deduplication. *(needs: traffic-store, connectivity)*
- [ ] **adsb** — ADS-B In traffic (GDL90 parsing, PowerFLARM/Stratux).
      *(needs: traffic-store)*

## Logging & recording

- [ ] **igc-write** — IGC recording: headers, B-records, pre-takeoff
      buffer, auto start/stop, interval control. Crash-safe: incremental
      flush-per-batch writes plus state snapshots so an interrupted flight
      resumes logging on restart. *(needs: igc-read, flight-modes)*
- [ ] **g-record** — tamper-evident G-record signing and validation.
      *(needs: igc-write)*
- [ ] **markers-pev** — manual/automatic markers and pilot events (1 Hz
      burst logging), markers on map. *(needs: igc-write, frontend-map)*
- [ ] **replay-ui** — flight replay controls in the UI (file picker,
      speed, seek) on top of the replay engine. *(needs: replay,
      frontend-protocol)*
- [ ] **engine-monitoring** — ENL/MoP detection, engine hours,
      microphone-based ENL. *(needs: igc-write)*

## Map & UI polish

- [ ] **map-orientation** — track-up / north-up / target-up, auto-zoom,
      circling zoom, smart offset position. *(needs: frontend-map,
      flight-modes)*
- [ ] **snail-trail** — flight trail with length modes and colouring by
      vario/altitude/speed. *(needs: frontend-map, vario-values)*
- [ ] **datafield-pages** — multiple data-field pages/layouts,
      per-flight-mode auto switching, bottom nav-box bar. *(needs:
      datafields-v1, flight-modes)*
- [ ] **units-settings** — per-quantity unit configuration UI wired
      through all displayed values. *(needs: datafields-v1)*
- [ ] **settings-persistence** — configuration profiles (per pilot/per
      aircraft), settings persistence adapter, profile switching. *(needs:
      core-state)*
- [ ] **aircraft-profiles** — plane profiles: polar selection, custom
      coefficients, ballast/weights, registration/comp ID. *(needs:
      settings-persistence, glide-settings)*
- [ ] **themes** — day/night/high-contrast modes, sunlight-readability
      contrast targets validated outdoors, auto-brightness hooks.
      *(needs: frontend-scaffold)*
- [ ] **i18n** — localization scaffolding (Paraglide JS) + German
      translation; land before untranslated strings accumulate.
      *(needs: frontend-scaffold)*
- [ ] **disclaimer** — first-run "not a certified navigation source"
      dialog and about-screen text. *(needs: frontend-scaffold)*
- [ ] **input-gestures** — configurable hardware buttons/keys and gesture
      bindings. *(needs: frontend-protocol)*
- [ ] **status-pages** — flight / times / system status dialogs.
      *(needs: datafields-v1)*
- [ ] **sun-ephemeris** — `libs/updraft_sun`: sunrise/sunset/twilight
      math; time-of-day data fields and "arrival past sunset" warning.
      *(needs: units, task-calculator)*
- [ ] **checklists** — user checklist files/pages. *(needs:
      frontend-protocol)*
- [ ] **weight-balance** — W&B / CG-envelope calculator. *(needs:
      aircraft-profiles)*
- [ ] **config-sharing** — configuration sharing via files / QR codes.
      *(needs: settings-persistence)*
- [ ] **stopwatch-misc** — stopwatch, position/ATC report page. *(needs:
      datafields-v1)*
- [ ] **ahrs-pfd** — attitude indicator / PFD from AHRS data; synthetic
      vision later. *(needs: nmea-airdata, io-adapters)*

## Online services

- [ ] **connectivity** — online/offline detection and state in core,
      offline-first hooks (status indicator, queue-and-retry for uploads).
      *(needs: core-state)*
- [ ] **basemap-packs** — offline basemap packs (PMTiles or MBTiles, format
      TBD) stored on device, served to MapLibre through the bulk geodata
      path. *(needs: bulk-data, frontend-map)*
- [ ] **data-downloads** — in-app download manager for waypoint / airspace
      / map / DEM data with repository manifest and offline caching.
      *(needs: connectivity)*
- [ ] **metar-taf** — METAR/TAF fetch, decode, map flags, QNH extraction.
      *(needs: core-state, frontend-map)*
- [ ] **weather-overlays** — rain radar and satellite imagery overlays
      with time slider; forecast overlays (SkySight/TopMeteo) behind the
      same interface. *(needs: frontend-map)*
- [ ] **wind-aloft** — multi-level forecast wind + live station wind
      display. *(needs: weather-overlays, wind-circling)*
- [ ] **notam** — NOTAM download rendered as airspace, filters, details.
      *(needs: airspace-store)*
- [ ] **task-download** — task download from SoaringSpot / WeGlide.
      *(needs: task-files)*
- [ ] **task-sharing** — task sharing via QR code / file share. *(needs:
      task-files)*
- [ ] **live-tracking** — position upload to OGN / SkyLines / LiveTrack24
      style services. *(needs: flight-modes, connectivity)*
- [ ] **contest-upload** — one-tap post-flight upload (WeGlide, OLC, …).
      *(needs: igc-write, connectivity)*
- [ ] **thermal-hotspots** — crowd-sourced thermal hotspot overlays
      (kk7 / WeGlide). *(needs: frontend-map)*
- [ ] **charts** — approach charts / georeferenced chart overlays.
      *(needs: frontend-map, data-downloads)*
- [ ] **cloud-sync** — settings/task/waypoint sync via third-party cloud
      services. *(needs: settings-persistence, connectivity)*
- [ ] **datalink-weather** — FIS-B / SiriusXM datalink weather. *(needs:
      weather-overlays, adsb)*

## Analysis & contest

- [ ] **barograph** — altitude trace page with working-band estimation.
      *(needs: igc-write, frontend-protocol)*
- [ ] **climb-stats** — per-climb history, thermal statistics, leg
      statistics pages. *(needs: thermal-assistant, task-engine)*
- [ ] **analysis-pages** — wind vs altitude, glide polar analysis, vario
      histogram, temperature trace. *(needs: barograph)*
- [ ] **cross-section** — airspace + terrain side-view profile ahead.
      *(needs: agl-terrain, airspace-store)*
- [ ] **contest-optimizer** — `libs/updraft_contest`: OLC/WeGlide/FAI rule
      sets, optimal path over the flown trace. *(needs: geo)*
- [ ] **live-scoring** — in-flight optimization: score/achieved-distance
      data fields, optimal path display. *(needs: contest-optimizer,
      datafields-v1)*
- [ ] **task-analysis** — post-flight per-leg statistics and AAT
      rendering. *(needs: task-engine, barograph)*

## Devices & platforms

- [ ] **serial-adapter** — serial/TTY adapter for desktop platforms with
      baud probing. *(needs: io-adapters)*
- [ ] **terminal-monitor** — terminal monitor page for I/O debugging. *(needs: io-adapters)*
- [ ] **devmode** — hidden developer mode (seven-tap unlock): byte-capture
      replay transport through the real parser stack, map rendering and
      data loading debug options. *(needs: frontend-protocol, io-adapters)*
- [ ] **bluetooth** — Bluetooth SPP adapter via Tauri plugin (per-platform permissions). *(needs: io-adapters, tauri-scaffold)*
- [ ] **ble** — Bluetooth BLE adapter via Tauri plugin (per-platform permissions). *(needs: io-adapters, tauri-scaffold)*
- [ ] **usb-otg** — USB-serial adapter via Android OTG. *(needs:
      serial-adapter, tauri-mobile)*
- [ ] **internal-sensors** — internal GPS and pressure sensor input via
      Tauri plugins, injected as typed messages, ranked below external
      devices; always-on by default (WeGlide-valid IGC logs) with a
      battery-saver setting. *(needs: core-time, tauri-scaffold)*
- [ ] **device-manager** — devices screen (user-ordered priority list),
      multi-device value merging, priority/fallback, NMEA
      pass-through/output. *(needs: io-adapters, gps-status)*
- [ ] **device-configs** — named device-config snapshots (device entries +
      priority order), aircraft-config linkage, manual save/load. *(needs:
      device-manager, aircraft-profiles)*
- [ ] **vendor-protocols** — driver/personality framework: sentence-family
      drivers, bidirectional settings sync with per-setting preferences,
      one-shot outbound operations, exclusive binary sessions. *(needs:
      device-manager)*
- [ ] **lxnav-sync** — LXNav personality: sync of MacCready, ballast, bugs,
      and QNH via `$PLXV*`. *(needs: vendor-protocols)*
- [ ] **flarm-declaration** — FLARM task declaration *(needs: vendor-protocols, task-files)*
- [ ] **lxnav-igc** — IGC file download from LXNav devices. *(needs:
      vendor-protocols)*
- [ ] **flarm-igc** — IGC file download from FLARM devices (FLARM, LX) via
      exclusive binary session. *(needs: vendor-protocols)*
- [ ] **tauri-mobile** — Android/iOS builds: location permission,
      background execution, keep-awake. *(needs: tauri-scaffold)*
- [ ] **sim-mode** — on-device simulator mode (fly without GPS): manual
      flying controls, direct position/altitude setting; activating
      sim/replay disables IGC logging and online data (weather, OGN).
      *(needs: replay)*
- [ ] **secondary-clients** — primary/secondary operation: auth, roles &
      permissions for remote frontends, repeater display mode. *(needs:
      server-protocol, settings-persistence)*
- [ ] **audio-alerts** — native audio plugin for airspace/traffic warning
      playback, driven directly from the core; ships with the first release
      so airspace warnings are audible from day one. *(needs:
      airspace-warnings, tauri-scaffold)*
- [ ] **battery-monitoring** — internal/external battery and voltage
      state. *(needs: device-manager)*
- [ ] **switch-inputs** — gear/flap warning digital inputs. *(needs:
      device-manager)*
- [ ] **radio** — radio frequency management via drivers. *(needs: vendor-protocols)*
- [ ] **xpdr** — transponder control via drivers. *(needs: vendor-protocols)*
- [ ] **two-seat** — front/rear cockpit sync of MC/ballast/target/wind.
      *(needs: secondary-clients, vendor-protocols)*
- [ ] **physiological** — heart rate / SpO₂ sensor input. *(needs: ble)*
- [ ] **audio-vario** — continuous audio vario via parameter-driven tone
      synthesis on the native audio thread (core streams climb rate).
      *(needs: audio-alerts, vario-values)*

## Distribution

- [ ] **releases** — packaging and release pipeline: GitHub Releases,
      Google Play, Apple App Store, F-Droid; platform-native update
      channels, no self-updater. Play Console foreground-service
      justification + demo video prepared before first submission.
      *(needs: tauri-mobile, disclaimer)*

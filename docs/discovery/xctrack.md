# Discovery: XCTrack (XCTrack Pro)

## System overview

- **Version(s) examined:** ~0.9.7.6 (config dump `versionCode 90760`) cross-referenced with documentation for the 0.9.8–0.9.13 beta line (SkySight, SafeSky/EC, obstacle warnings, multi-sensor fallback). "Pro" is the same binary with paid features unlocked.
- **Date(s) examined:** 2026-07-06
- **Platform(s):** Android only (5.0+; legacy builds to 2.2). No iOS/desktop. Ships pre-installed on AIR³ hardware.
- **License:** Proprietary / closed-source freeware. The **main app source is not published** — only the _competition/task interface_ spec, the _XCTrackOpenData_ NMEA spec, the public issue tracker (`gitlab.com/xcontest-public/xctrack-public`, issues only), and community translations are public. Pro is a paid unlock. This analysis is therefore based on the public format specs, real exported `.xcfg`/`.xctsk` files, the third-party layout-editor's embedded widget catalog, and the official + AIR³ documentation — **not** on primary source code.
- **Offline behavior:** Core flying works fully offline — vector/terrain maps, OpenAir airspace, task navigation, vario, wind computation, IGC logging, and FANET/FLARM traffic (via radio module) need no connectivity. Internet is required only for: XContest livetracking + messaging, SkySight overlays, rain radar, thermal-map (kk7) download, web airspace updates, one-click XContest upload/auto-claim, and the GPS week-rollover time fix.
- **Configuration model:** JSON `.xcfg` config files (import/export, QR share, community library) holding per-orientation page lists, each with widgets carrying absolute grid geometry (`X1/Y1/X2/Y2` in 1/10000 units), UUID, theme and per-widget options. Layered on top: four "pilot profiles" (Kiss/Easy/Expert/Paramotor), global preferences (units, themes, key bindings, sensors), and per-widget theme overrides. Highly configurable, multi-page, portrait+landscape.

## Feature inventory

**Status legend:**

- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

### Map display & interaction

- `●` Vector / topographic map rendering: Mapsforge-compatible road maps + custom XML render themes (Clearpilot, XContest style, etc.).
- `●` Terrain shading / elevation: separate terrain/elevation data used for relief and AGL; missing tiles render in page background color.
- `●` Map orientation (track / north / target up): North, Bearing (track), Heading, Navigation-target, Downwind, and Travel-direction up — all selectable.
- `◐` Auto-zoom & circling zoom (mode-dependent): "smart pilot position" shifts the pilot off-center by 20-min vs XC speed; auto-switch to the thermal-assistant page on circling. No documented continuous auto-zoom / dedicated circling-zoom scale.
- `●` Manual pan & zoom: pinch-zoom, dedicated pan mode (swipe up on map), zoom +/- widgets and volume-key zoom.
- `●` Snail trail / flight trail: own tracklog drawn with configurable custom color.
- `●` Glide range ("reachability") line: "Mark glide distance" draws a solid circle (from real glide ratio) and a dotted/empty circle (from configured glide ratio + trim speed + computed wind); also rendered as glide lines in the side view.
- `◐` Thermal markers on map: thermal _overlay_ from thermal.kk7.ch (historical hotspots) and the thermal-assistant bubble view; no evidence of persistent own-climb thermal pins on the main map.
- `◐` Markers / pilot events on map: other live/FANET/FLARM pilots shown with labels; no clear user-placed event marker on the map beyond creating waypoints.
- `◐` Waypoint & landable symbology / labels: waypoints from files shown with names/labels (size configurable). No strong landable-vs-non-landable symbology (PG waypoint model).
- `◐` "What's here" query: long-click in pan mode → navigate-to point, create waypoint, or select an airspace zone to read/change its status. Functions as a context query rather than a generic "what's here" readout.

### Waypoints & navigation

- `●` Waypoint database: waypoint files in all major formats (CUP etc.), managed via a waypoint manager; can add waypoints in-flight.
- `◐` Landable vs non-landable distinction: not a first-class concept in the PG waypoint model; not documented.
- `?` Nearest (waypoint / landable / airfield): a dedicated "nearest landable/airfield" function is not documented; navigation is task/waypoint-driven.
- `●` Go-to / direct navigation: long-click "navigate to" any point/waypoint; next/prev waypoint stepping.
- `◐` Waypoint details (info, image, runway, freq): name/description/smoothed-altitude present; no runway/frequency/image fields (PG data model).
- `○` Alternates / safety-landing selection: not present.

### Cross-country tasks

- `●` Racing tasks: SSS type RACE with time gates; full PG race support.
- `◐` Assign Area Tasks (AAT): no gliding-style AAT. All turnpoints are cylinders and the route is optimized through them, which covers PG "area"-style needs, but there is no AAT min-time construct.
- `●` Cross-country task types (FAI triangle, out-and-return, DMSt, etc.): live FAI-triangle assistant and XContest free/triangle optimization; classic multi-TP tasks.
- `●` Observation zones & start/finish/turnpoint rules: TAKEOFF / SSS / ESS / goal roles; cylinders (enter/exit for start), goal LINE or CYLINDER, takeoff open/close, goal deadline. FAI-sphere vs WGS84 distance model.
- `●` Task manager (build / edit): full turnpoint/edit screen with task board showing min distance and distance-through-centers.
- `●` Task calculator (in-flight required speeds etc.): speed-to-start, time-to-start, %-of-speed-section, glide/altitude/distance to goal & ESS.
- `●` In-flight task edit: turnpoints and waypoints editable during flight; can insert new organizer waypoints.
- `◐` FAI badge / record support: FAI _assistant_ for triangles and XContest record/claim workflow; not an IGC-approved recorder for CIVL record levels (see 3.16).
- `●` Task import / export / declaration to logger: `.xctsk` files, QR code, NFC, email, WhatsApp; QR-compressed variant. Declaration to an external approved logger is out of scope (phone is the recorder).

### Glide computer

- `◐` Flight modes + auto display switching (cruise/circling/final): automatic actions switch pages (e.g. to thermal assistant on circling and back). No explicit cruise/final-glide "mode" state machine as in glider computers.
- `○` MacCready setting (manual): not present (PG model; uses trim speed + glide ratio instead).
- `○` Auto MacCready (modes): not present.
- `○` Glide polar (library + custom): no polar. A single trim-speed + glide-ratio pair per aircraft substitutes for a polar.
- `○` Ballast / bugs / weight setup: not present.
- `◐` Speed to fly / speed command / risk factor: no classic MacCready speed-to-fly. A "speed to make start" and glide-speed-vs-wind figure exist in the competition context.
- `◐` Safety heights / safety MacCready: per-widget configurable altitude reserve for required-glide computations; no safety-MacCready concept.
- `●` Final glide calculator (wind, altitude required): glide/altitude-over-goal and -ESS, required glide ratio to next TP/goal, using GPS altitude, configured glide ratio and computed wind.
- `●` Task speed estimation: speed-to-start, XC speed, %-of-speed-section; XC speed usable for FAI-triangle hunting.
- `●` Optimal cruise track: XContest live optimization computes the optimized point/route; "navigate to optimized point" is the default target.

### Atmosphere & instruments

- `●` Variometer display: numeric vario, text vario, and a colored lift/sink bar column.
- `●` Average climb: configurable averaging interval (often set to half/one turn) as an integrator vario.
- `●` Audio variometer: acoustic vario with configurable/custom sounds, dynamic tones, and a weak-lift ("sniffer") double-beep; mute widget.
- `●` Air-data inputs (external sensors): external pressure, GPS, airspeed and wind can be taken from connected instruments.
- `●` Wind estimation (circling / zigzag / compass / external): computed per full circle during thermalling; airspeed derived from wind + groundspeed; can be overridden by external sensor.
- `◐` Wind display & manual override: wind speed/direction widgets, arc/arrow styles; external override and 180° reversal available. No manual "set wind by hand" documented.
- `●` Thermal locator / assistant / centering aid: dedicated Thermal Assistant page + widget with bubble-strength visualization; auto-engaged on circling.
- `◐` Thermal profile: altitude-statistics widget graphs thermal strength vs altitude (also wind and groundspeed variants).
- `◐` Convection forecast (cloud base etc.): via SkySight overlays (Cu cloudbase, thermal strength, etc.) when logged in; not an internal model.

### Weather

- `◐` METAR / TAF: QNH can be pulled from METAR (or a weather station); no full METAR/TAF text display.
- `●` Forecast overlays (SkySight / TopMeteo / RASP): SkySight integration — thermal strength, Cu cloudbase, XC speed, convergence, thermal wind layers with opacity and forecast-time (now → +4h) selection. (SkySight only, not TopMeteo/RASP.)
- `◐` Wind aloft / weather-station data: SkySight forecast wind barbs on the map; QNH from a weather station. No numeric multi-level wind-aloft table.
- `◐` In-flight weather updates: live rain-radar overlay and (with connectivity) SkySight; otherwise pre-downloaded.

### Airspace

- `●` Airspace display (classes, filtering): OpenAir files plus automatic web airspace from airspace.xcontest.org (per-country), with temporary activations. Rendered on all map widgets.
- `●` Proximity / incursion warnings + acknowledgement: airspace-proximity widget with distance to nearest zone; restricted (red) vs warning (orange) handling; per-zone status can be changed (auto/active) and acknowledged; configurable postponing of high-floor zones and split display; option to force GPS altitude for infringement checks.
- `●` Airspace query / details: long-click a zone to review details / change status.
- `◐` NOTAM handling: temporary airspace _activations_ are supported via the web airspace source; no general NOTAM feed/parser.

### Traffic & collision awareness (FLARM)

- `●` FLARM Traffic on map: parses FLARM `$PFLAA`; shows other aircraft from an internal (AIR³+) or external FANET/FLARM module.
- `●` OGN Traffic on map: OGN and other live networks appear (natively via XContest livetracking; broader networks via XC Guide TCP feed).
- `◐` FLARM and OGN Traffic deduplication: an "EC ID" pairing table merges FLARM/FANET/livetracking identities to a single username (auto or manual); effectively dedups per-pilot but is identity-centric rather than track-fusion.
- `○` FLARM radar view: no dedicated relative-bearing radar dial; traffic is shown on the moving map only.
- `?` FLARM traffic dead reckoning: not documented.
- `◐` Collision / proximity warnings: primarily a "see nearby pilots" / electronic-conspicuity display rather than a certified collision-warning instrument; no documented FLARM-style predicted-collision alarm in-app.
- `●` Traffic details dialogue: per-contact info — EC ID, username, altitude, AGL, take-off site, distance flown, distance from you.
- `●` Traffic ID / registration lookup: EC-ID→username pairing (via FANET, XContest, or manual entry), including address-type handling (ICAO/FLARM/random).
- `◐` Team flying / buddy codes: select pilots to display and message them via livetracking; buddy-style tracking of chosen FANET/FLARM IDs. No numeric FLARM "team code" scheme.

### Avionics & airframe

- `●` Battery / voltage monitoring: device battery in the status line / battery widget.
- `●` GPS status / connection / altitude source: GPS status widget/status-line; selectable internal/external GPS and baro source (with a source-switch widget).
- `○` Engine / powered flight (ENL, MoP, engine hours): a Paramotor pilot profile exists, but no ENL/MoP/engine-hour instrumentation.
- `○` Radio frequency management / control: not present.
- `○` Transponder (XPDR) control / squawk: not present.
- `○` Switch inputs (gear / flap warnings etc.): not applicable / not present.
- `●` Multiple external devices / slave mode: multi-sensor connectivity with automatic fallback; NMEA output can drive other devices; ActiveLook AR glasses supported as a display slave.

### Data fields (InfoBox system)

- `●` Configurable data-field grid: free-placement widgets with per-widget geometry, border, background transparency, title, and theme.
- `●` Multiple data-field pages / layouts: unlimited pages per orientation; separate portrait/landscape layouts.
- `◐` Per-flight-mode auto layout: pages can be restricted to specific navigation types and auto-switched (e.g. thermal-assistant page on circling); not a full per-mode InfoBox auto-swap.
- `●` Altitude / altimetry values (QNH, pressure alt, FL, AGL): GPS altitude, baro altitude (QNH), AMSL, Flight Level (1013.25), AGL, altitude-above-takeoff, QNH widget.
- `●` Speed values (GS / TAS / IAS / optimal): groundspeed, estimated airspeed (from wind), glide-speed, XC speed.
- `●` Direction values (track / bearing / heading): bearing/track, computed or compass heading, compass-and-wind and digital-compass widgets.
- `●` Time values (UTC / local / ETA / ETE / sunset / flight time): time, air time, ETA/next-TP time-of-arrival, sunset, civil twilight.
- `●` Touch / gesture interaction with data fields: touchable action widgets (zoom, nav, camera, phone, brightness, vario mute), swipe menu, key bindings.

### Analysis & review

- `◐` Barograph / altitude trace: in-flight altitude-statistics and vertical-graph widgets; no dedicated post-flight barograph screen (post-flight analysis lives on XContest).
- `◐` Climb history / thermal analysis: thermal-strength statistics graph in-flight.
- `◐` Wind analysis: wind strength/direction statistics graph.
- `○` Glide polar analysis: not present (no polar model).
- `◐` Task analysis: task-summary widget in-flight; full scoring/analysis on the XContest server.
- `◐` OLC / contest analysis: live XContest optimization in-flight (distance/speed/route); complete contest analysis on the web after upload.
- `●` Airspace cross-section: side-view widget gives a 3-D/vertical understanding of airspace ahead, with glide lines.

### Contest / WeGlide optimization (live)

- `●` Live WeGlide optimization in-flight: live _XContest_ optimization in-flight (FAI sectors, free/triangle). XContest, not WeGlide specifically.
- `●` Flight trace maintenance: maintains the optimized trace/route continuously during flight.
- `●` Live scoring / achieved distance: flown-XC-distance, unfinished-triangle, and average-XC-speed widgets.

### Live tracking

- `●` Live tracking upload (OGN / SkyLines / cloud): native XContest livetracking (position every ~60 s, ~100 KB/h); OGN, Livetrack24, SkyLines, XC Globe and others via XC Guide; SafeSky electronic conspicuity.
- `●` Retrieve / crew comms / position sharing: see other live pilots (with altitude/AGL/site/distance), select whom to display, and send/receive text messages; auto position-sharing enabled after detected take-off; optional auto flight-claim after landing.

### Instrument & device connectivity (I/O)

- `●` Bluetooth: classic BT sensor support.
- `●` BLE: BLE with service/characteristic discovery for many PG varios (XCTracer, BlueFly, Skydrop, LeBip, Vector, MipFly, etc.).
- `◐` Serial: no native RS-232; reachable only via USB-serial adapters / device-specific links.
- `●` USB: USB sensor connectivity; USB NMEA-style output.
- `●` TCP/IP / network: TCP client and UDP (e.g. XC Guide feed on localhost:10110).
- `●` NMEA input parsing: parses FLARM `$PFLAA`, FANET `$FNNGB`, plus vario/GPS/baro/wind/airspeed sentences.
- `●` NMEA / data output (drive other devices): XCTrackOpenData `$XCTOD` custom sentences over BLE/BT/USB/TCP/UDP; drives ActiveLook glasses.
- `●` Positioning source selection (internal / external): choose internal vs external for GPS and baro, with priority ordering and automatic fallback.
- `●` Vendor protocols (LXNav / FLARM / …): FLARM + FANET + many PG-vario BLE profiles. PG-centric; LXNav/gliding avionics protocols not targeted.

### Data & file management

- `●` Waypoint files (CUP / …): all major waypoint formats.
- `◐` Airspace files (OpenAir / CUB / …): OpenAir files + web source. No CUB/other binary formats noted.
- `●` Terrain / topology / map data: Mapsforge road maps + terrain/elevation; custom render themes.
- `●` In-app data download & updates: web airspace auto-update, map/terrain download, thermal-map & rain-radar fetch; dev-build/localization updater.
- `◐` Aircraft / plane profiles (polar, ballast, reg, comp ID): a single aircraft with name (for XContest), trim speed and glide ratio. No polar/ballast; no multi-aircraft profile switcher beyond pilot profiles.
- `○` Handicap / polar lists: not present.

### Logging & recording

- `●` IGC flight log recording: records IGC.
- `◐` Approved / signed logger (badge / record): produces IGC accepted by XContest; using an external GPS/baro, or the GPS-week-rollover time fix, makes the log non-FAI-CIVL-compliant (not eligible for top-level comps/records). Not on the IGC-approved recorder list.
- `●` Flight replay: tracklogs can be replayed (up to ~3× for the smart-position logic to behave).
- `?` Pilot events / markers logging: dedicated IGC pilot-event marker not documented (camera/phone action widgets exist but aren't log markers).

### Configuration & UI

- `●` Configuration profiles (per user / per aircraft / global): `.xcfg` config files (import/export, QR, community library) + four pilot profiles (Kiss/Easy/Expert/Paramotor).
- `●` Screen layout / data-field geometry: absolute grid geometry per widget; multi-page, dual-orientation; visual community editor exists (3rd-party).
- `●` Day / night / high-contrast modes: five themes including Black (power-saving), White (map readability) and eInk; per-widget theme override.
- `●` Units: configurable altitude, speed, vertical-speed, distance, etc.
- `●` Input / gestures / hardware buttons: key bindings, volume-button paging/zoom, proximity sensor as a hardware key, touch action widgets.
- `●` Language / localization: 20+ community-translated languages; per-map language override.

### Additional features (outside the shared taxonomy)

- `●` Web-page widget: renders a live web page as a widget with `${lat}`/`${lng}` placeholders for in-flight web content.
- `●` In-app pilot messaging: "incoming messages" widget + send messages to livetracking pilots.
- `●` Electronic Conspicuity (SafeSky): be-seen/see-others conspicuity integration with the EC-ID pairing table.
- `●` Obstacle / hazard mode: obstacle data and warnings (cables/obstacles) for AT/FR/DE/IT/SI/CH, with a map hazard mode.
- `●` ActiveLook AR-glasses output: multishaded map/HUD to ActiveLook smart glasses.
- `●` Thermal hotspot overlay: thermal.kk7.ch historical thermal map (basic and season/time-adjusted).
- `●` Rain-radar overlay: live precipitation radar layer.
- `●` Automatic actions: auto page-switching, auto position-sharing on take-off, auto XContest flight-claim after landing, auto master-sync-off in flight.
- `●` Task sharing over QR / NFC / email / WhatsApp: including QR-compressed task encoding.
- `●` Action widgets: one-touch camera launch, phone-call-to-contact (hands-on-brakes), brightness +/- (autonomy management), vario mute, menu, zoom, next/prev waypoint.
- `●` GPS week-rollover time fix: corrects buggy-firmware GPS time against an internet time source (flags the log non-CIVL).

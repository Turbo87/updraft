# Discovery: Features

This document contains a single categorized list of all features and subfeatures found across the analyzed systems, compiled from the discovery documents in this directory.

Each feature is prefixed with a status marker, so we can triage which ones to target for our app:

- ✅ target
- ✳️ maybe / partial
- ❌ skip
- ⬜ undecided

## Map display & interaction

- ✅ Vector / topographic map rendering (client-side vector, e.g. shapefile/Mapsforge/MapLibre; or pre-rendered raster tiles)
- ✳️ Raster / satellite / sectional-chart basemaps _(SeeYou Navigator, Navia, G3X, Enroute)_ (partial: only for satellite/weather for now)
- ❌ Custom map render themes / styles _(XCTrack)_
- ✅ Terrain shading / elevation (DEM colouring, hillshade/slope shading, colour schemes, e-ink/greyscale rendering)
- ✳️ Map orientation (track / north / target up; plus variants: heading-up, goal-up, downwind-up, desired-track-up, separate circling orientation) (partial: just track / north / target up)
- ✅ Auto-zoom & circling zoom (mode-dependent)
- ✅ Smart / offset pilot position on screen _(XCTrack, XCSoar)_
- ✅ Manual pan & zoom (gestures, buttons, knobs, dedicated pan mode)
- ✅ Snail trail / flight trail (length modes, colouring by vario/altitude/speed, wind-drift compensation, engine-on colouring)
- ✅ Glide range ("reachability") line / area (terrain-aware footprint, turning reach, safety-MC based fill, glide circles)
- ✅ Thermal markers on map (own-climb thermal history, drift-compensated sources)
- ✅ Thermal hotspot overlays (historical/crowd-sourced, e.g. thermal.kk7.ch, WeGlide live thermals) _(SeeYou Navigator, XCTrack, WeGlide Copilot)_
- ✅ Markers / pilot events on map (manual & automatic markers, PEV)
- ✅ Waypoint & landable symbology / labels (icon sets, reachability colouring, runway direction, declutter, arrival-height labels)
- ✅ "What's here" query (multi-object tap/pan query with details)
- ⬜ Multimaps / split-screen map views (map + sideview / terrain / airspace) _(LK8000)_
- ⬜ Visual Glide: horizontal strip of glide bars to nearby landables _(LK8000)_
- ⬜ 3D synthetic map view (terrain with rivers/roads/airspace/traffic traces) _(LXNAV, G3X Synthetic Vision)_

## Waypoints & navigation

- ✅ Waypoint database (multiple simultaneous files, unlimited points, search)
- ✅ Landable vs non-landable distinction (incl. outlanding-field layers with quality grading)
- ✅ Nearest (waypoint / landable / airfield) lists and sorting
- ✅ Go-to / direct navigation
- ✅ Waypoint details (info text, images, runway, frequency, elevation)
- ✅ Alternates / safety-landing selection (alternates dialog, abort mode, best-alternate)
- ✅ Multi-target / secondary-target navigation ("X>", alternate arrival readouts) _(LK8000)_
- ⬜ Oracle-style "what's useful near me" assistant _(LK8000)_
- ⬜ Approach assistant / runway selection for landables _(LK8000)_
- ⬜ 3D route planning around terrain and airspace _(XCSoar)_
- ✅ GA flight-route editor (leg-based nav routes as distinct from scored tasks) _(Enroute, G3X flight plans)_

## Cross-country tasks

- ✅ Racing tasks (start arm/gates, start speed/height limits, PEV start)
- ✅ Assign Area Tasks (AAT / MAT / TAT; isolines, target moving, min task time)
- ✅ Cross-country task types (FAI triangle, out-and-return, DMSt, Grand Prix, national variants)
- ✅ Observation zones & start/finish/turnpoint rules (cylinders, FAI sectors, keyholes, lines, ESS/goal semantics, per-point overrides)
- ✅ Task manager (build / edit; list and map editing; templates)
- ✅ Task calculator (in-flight required speeds, AAT range/what-if, time gates)
- ✅ In-flight task edit
- ✅ FAI badge / record support (badge/record rules, finish-below-start handling)
- ✅ FAI triangle assistant (live triangle-closing guidance) _(LXNAV, SeeYou Navigator, XCTrack, LK8000 FAI Assistant)_
- ✅ Task import / export / declaration to logger (file formats, declaration to external loggers/internal recorder)
- ✅ Task download from online sources (SoaringSpot, WeGlide, cloud collections)
- ✅ Task sharing via QR code / NFC / email / messaging _(XCTrack, SeeYou Navigator)_

## Glide computer

- ✅ Flight modes + auto display switching (cruise / circling / final glide / abort)
- ✅ MacCready setting (manual, incl. sync with connected vario)
- ✅ Auto MacCready (final-glide, average-climb, suggestion modes)
- ✅ Glide polar (library + custom coefficients)
- ✅ Ballast / bugs / weight setup (incl. ballast dump timer, tail water)
- ✅ Speed to fly / speed command / risk factor (STF chevrons/audio, dolphin speed, required STF)
- ✅ Safety heights / safety MacCready (arrival safety height, terrain clearance, safety-MC offset)
- ✅ Final glide calculator (wind-corrected altitude required, dual Mc/Mc-0 displays)
- ✅ Task speed estimation (achieved/required task speed)
- ✅ Optimal cruise track (optimal track indicator, AAT optimal arrow, optimized-point navigation)

## Atmosphere & instruments

- ✅ Variometer display (needle/tape/numeric/bar; netto, relative, TE)
- ✅ Average climb (integrator, thermal average, last/all-thermal statistics)
- ⬜ Audio variometer (tone ramps, dead band, equalizer, sniffer/weak-lift tones, mute)
- ✅ Air-data inputs (external sensors: baro, TAS/IAS, TE vario, OAT, humidity, load factor/IMU)
- ⬜ Inertial/aerodynamic air-data models (HAWK: inertial wind, netto, AoA, sideslip) _(LXNAV; Navia "inertial vario/wind")_
- ✅ Wind estimation (circling / zigzag / compass / combination / external; layered wind statistics)
- ✅ Wind display & manual override (vectors, per-layer editing, freehand draw)
- ✅ Thermal locator / assistant / centering aid (graphical + audible)
- ✅ Thermal profile (thermal band / climb-vs-altitude graphs)
- ⬜ Convection forecast (cloud base / convection ceiling from temp trace or forecast products)
- ✅ Pressure-altitude / cabin-altitude tool _(Enroute)_
- ✅ Density-altitude computation _(Enroute, G3X)_
- ✅ Potential-temperature trigger aid _(LXNAV)_
- ✅ Physiological sensors: heart rate, SpO₂ _(LXNAV, LK8000, G3X)_

## Weather

- ✅ METAR / TAF (decoded display, map flags, QNH extraction, meteograms)
- ✅ Forecast overlays (SkySight / TopMeteo / RASP / pc_met; opacity, forecast-time animation)
- ✅ Wind aloft / weather-station data (multi-level forecast wind, live station wind, FANET weather beacons)
- ✅ In-flight weather updates (live refresh over connectivity, cached weather after signal loss)
- ✅ Rain radar overlay _(WeGlide Copilot, XCTrack, SeeYou Navigator, LXNAV)_
- ✅ Live weather-satellite imagery (with time slider/animation) _(WeGlide Copilot, LXNAV, SeeYou Navigator)_
- ✅ Datalink weather (SiriusXM / FIS-B: NEXRAD, echo tops, lightning, PIREPs, AIRMET/SIGMET, TFRs) _(G3X)_

## Airspace

- ✅ Airspace display (classes, per-class styling/filtering, altitude filters, inactive/periodic zones)
- ✅ Proximity / incursion warnings + acknowledgement (predicted incursion, graded warnings, per-zone/group dismiss with duration)
- ✅ Airspace query / details (vicinity lists, details dialogs, per-zone enable/disable, in-app editing)
- ✅ NOTAM handling (live NOTAM download as airspace, filters, details; regional TFR/DABS products)
- ❌ Post-flight airspace-crossing scan/review _(WeGlide Copilot)_

## Traffic & collision awareness

- ✅ FLARM traffic on map (threat colouring, labels, tracked paths)
- ✅ OGN / internet traffic on map (live network traffic)
- ✅ FLARM and OGN traffic deduplication (track fusion or identity pairing)
- ✅ FLARM radar view (dedicated relative-position rose/page)
- ✳️ FLARM traffic dead reckoning / animation between updates (maybe for 1-3 seconds)
- ✅ Collision / proximity warnings (alarm levels, voice callouts, obstacle & alert-zone warnings)
- ✅ Traffic details dialogue (per-target ID, vario, relative position, sortable lists)
- ✅ Traffic ID / registration lookup (FlarmNet, OGN DDB, transponder DB, custom naming)
- ✳️ Team flying / buddy codes (encrypted team codes, buddy starring/highlighting, friend colouring) (partial: no team codes)
- ✅ ADS-B In traffic (via PowerFLARM/Stratux/dual receivers or TIS/TAS) _(XCSoar, LXNAV, Navia, G3X)_
- ⬜ TIS-A / TAS / TCAS traffic with TCAS symbology and voice advisories _(G3X)_
- ⬜ PCAS non-directional target display _(LXNAV)_
- ⬜ FANET support (traffic, weather beacons, device naming) _(LK8000, XCTrack, SeeYou Navigator/Oudie Fanet+)_
- ⬜ Electronic conspicuity integration (SafeSky; EC-ID pairing across networks) _(XCTrack)_

## Avionics & airframe

- ✅ Battery / voltage monitoring (internal + external batteries, battery-type profiles, per-cell monitors)
- ✅ GPS status / connection / altitude source (satellite views, multi-source failover, geoid correction, baro-source arbitration)
- ✅ Engine / powered flight (ENL, MoP, engine hours, fuel model, e-Glide energy reference)
- ⬜ Full piston-engine indication (EIS: RPM, fuel flow/calculator, CHT/EGT, Lean Assist, CAS messages) _(G3X; Navia EMU/MOP devices)_
- ✅ Microphone-based ENL recording into IGC _(WeGlide Copilot)_
- ✅ Radio frequency management / control (active/standby, dual watch, tune from waypoint, driver-based control)
- ✅ Transponder (XPDR) control / squawk (code/mode/ident via supported drivers)
- ✅ Switch inputs (gear / flap warnings, configurable digital inputs, AGL-triggered reminders)
- ✅ Multiple external devices / slave mode (device slots, value merging, NMEA pass-through, repeater displays)
- ✅ Two-seat / dual-cockpit operation (front/rear sync of Mc/ballast/target/wind) _(LXNAV, Navia dual displays & grips)_
- ✅ Weight & balance / CG-envelope calculator _(LXNAV, G3X)_
- ⬜ Electrical-system / electronic circuit-breaker integration (Vertical Power) _(G3X)_
- ✅ Carbon-monoxide detector integration _(G3X)_
- ❌ Uninterruptible / backup power management _(Navia UPS)_
- ✅ Analog & telemetry inputs (multi-channel analog acquisition, engine/battery telemetry) _(LXNAV LXDAQ/JRES)_

## Data fields (InfoBox / navbox / widget system)

- ✅ Configurable data-field grid (selectable geometries, free widget placement, styling)
- ✅ Multiple data-field pages / layouts (per-orientation layouts, import/export, community layout sharing)
- ✅ Per-flight-mode auto layout (mode-driven InfoBox sets, auto thermal page)
- ✅ Altitude / altimetry values (QNH, pressure altitude, FL, AGL, above-takeoff)
- ✅ Speed values (GS / TAS / IAS / optimal / XC speed)
- ✅ Direction values (track / bearing / heading / radial / steering course)
- ✅ Time values (UTC / local / ETA / ETE / sunset / twilight / flight time)
- ✅ Touch / gesture interaction with data fields (tap-to-edit, long-press editing, interactive buttons)
- ✅ Bottom bar with multi-mode nav-box stripes _(LK8000)_
- ⬜ Action widgets (camera, phone call, brightness, vario mute, zoom, page/waypoint switching) _(XCTrack)_
- ❌ Web-page widget (live web content with position placeholders) _(XCTrack)_

## Analysis & review

- ✅ Barograph / altitude trace (incl. working-band estimation)
- ✅ Climb history / thermal analysis (per-climb charts, thermal graphs, leg statistics)
- ✅ Wind analysis (wind vs altitude)
- ✅ Glide polar analysis
- ✅ Task analysis (per-leg statistics, AAT rendering)
- ✅ OLC / contest analysis (optimal path display, live score estimation)
- ✅ Airspace cross-section / side view (terrain + airspace profile ahead)
- ✅ Vario histogram and temperature-trace pages _(XCSoar)_
- ⬜ Post-landing statistics report _(SeeYou Navigator, WeGlide Copilot)_
- ⬜ Pilot logbook (auto-written, totals, photos/videos, manual entries) _(SeeYou Navigator)_
- ❌ CSV flight/engine data logging for off-board analysis _(G3X)_

## Contest optimization (live)

- ✅ Live contest optimization in-flight (OLC / WeGlide / XContest / DMSt / FAI rule sets)
- ⬜ Flight trace maintenance (continuous optimized-trace computation)
- ✅ Live scoring / achieved distance (score/distance data fields, live ranking)
- ✅ One-tap post-flight contest upload (OLC, WeGlide, XContest, DHV-XC) _(SeeYou Navigator, WeGlide Copilot, XCTrack auto-claim)_

## Live tracking

- ✅ Live tracking upload (OGN / SkyLines / LiveTrack24 / XContest / cloud services / FFVL / PureTrack / Traccar)
- ⬜ Retrieve / crew comms / position sharing (team position sharing, share-to-external-apps, live view of other pilots)
- ⬜ In-app pilot-to-pilot messaging over livetracking _(XCTrack)_
- ⬜ Automatic position-sharing on takeoff / auto flight claim after landing _(XCTrack)_
- ⬜ Location sharing to external apps (`geo:` URLs, share sheets) _(Enroute)_

## Instrument & device connectivity (I/O)

- ✅ Bluetooth (classic / SPP)
- ✅ BLE (incl. sensor service discovery)
- ✅ Serial (RS-232 / TTY / RS485 device bus)
- ✅ USB (USB-serial, memory-stick transfer)
- ✅ TCP/IP / network (TCP client/server, UDP, Wi-Fi modules)
- ❌ Built-in cellular connectivity (global SIM / LTE) _(Navia)_
- ✅ NMEA input parsing (position, air data, traffic; GDL90, X-GPS and vendor sentences)
- ✅ NMEA / data output (drive other devices, slave displays, custom output sentences)
- ✅ Positioning source selection (internal / external, priority ordering, automatic fallback)
- ✅ Vendor protocols (LXNAV, FLARM, CAI, Vega, Flymaster, XCVario, radio/XPDR bridges, and many more)
- ✅ Device management dialogs (device memory/setup management, e.g. CAI302, Vega) _(XCSoar)_
- ✅ IGC file download from connected devices (FLARM, LX, EOS/ERA) _(LK8000)_
- ✅ Terminal / COM-port monitor for I/O debugging _(LK8000)_
- ⬜ AR smart-glasses output (ActiveLook HUD) _(XCTrack)_
- ❌ Open hardware / documented integration protocols _(Navia)_

## Data & file management

- ✅ Waypoint files (CUP, CUPX, DAT, WPT/OZI, CompeGPS, OpenAIP, GPX, GeoJSON)
- ✅ Airspace files (OpenAir, CUB, SUA, OpenAIP; server-sourced databases)
- ✅ Terrain / topology / map data (map packages, MBTiles, DEM, scanned/raster charts)
- ✅ In-app data download & updates (region managers, repository manifests, web airspace auto-update)
- ✅ Aircraft / plane profiles (polar, ballast, registration, competition ID, FLARM ID, weight & balance)
- ✅ Handicap / polar lists (built-in polar stores, handicap factors)
- ⬜ GA route file interchange (GPX, FPL, PLN, TripKit) _(Enroute, G3X SD-card flight plans)_
- ✅ Cloud data sync (waypoints, tasks, maps, profiles, settings across devices) _(SeeYou Navigator, LXNAV Connect, LX Cloud/Navia)_
- ✅ Configuration sharing via files / QR codes / community libraries _(XCTrack)_

## Logging & recording

- ✅ IGC flight log recording (auto start/stop, pre-takeoff buffer, interval control)
- ✅ Approved / signed logger (IGC-certified recorders; tamper-evident G-records; cryptographic signing + device attestation)
- ✅ Flight replay (IGC/NMEA replay with variable speed)
- ✅ Pilot events / markers logging (PEV with 1 Hz bursts, marker files, logger announcements)

## Configuration & UI

- ✅ Configuration profiles (per user / per aircraft / per pilot / global; pin/admin locks, club mode)
- ✅ Screen layout / data-field geometry (layout editors, on-device and desktop styling tools)
- ✅ Day / night / high-contrast modes (themes, e-ink modes, auto-brightness, night-vision-friendly modes)
- ✅ Units (presets and per-quantity custom units, distance models)
- ✅ Input / gestures / hardware buttons (custom keys/menus, key bindings, remote sticks, knobs, haptics)
- ✅ Language / localization (bundled translations, community translation projects)
- ❌ Scripting / automation (embedded Lua engine; event-driven "automatic actions") _(XCSoar Lua, XCTrack automatic actions)_
- ✅ Status dialogues (flight / times / system / rules status views) _(XCSoar)_

## Flight instruments & EFIS

- ✅ Attitude indicator / artificial horizon (AHRS-driven) _(XCSoar, LXNAV, Navia, G3X)_
- ✅ Primary Flight Display (airspeed/altitude tapes, HSI, slip/skid) _(G3X, LXNAV PFD symbols)_
- ✅ Synthetic Vision (3D forward-looking terrain/obstacle/traffic/runway) _(G3X, LXNAV 3D view)_
- ⬜ Angle of Attack indication with aural alerting _(G3X, LXNAV HAWK)_
- ⬜ G-meter _(LXNAV)_
- ✅ Flap tape with suggested flap position _(LXNAV)_
- ❌ Autopilot / Automatic Flight Control System (flight director, lateral/vertical modes, coupled approaches) _(G3X)_
- ❌ Electronic Stability & Protection (envelope protection) _(G3X)_
- ✅ VNAV vertical navigation to altitude constraints _(G3X)_
- ⬜ IFR GPS navigation (airways, holds, procedures via external navigator) _(G3X)_

## Terrain & obstacle safety

- ⬜ Terrain proximity alerting (colour-coded terrain, terrain warnings) _(G3X, SeeYou Navigator terrain warnings)_
- ✅ Obstacle databases & warnings (worldwide or regional cable/obstacle data, hazard display modes) _(SeeYou Navigator, XCTrack, G3X, LXNAV FLARM obstacle warnings)_

## Charts & documents

- ✅ Visual approach charts / georeferenced chart overlays (VAC, TripKit) _(Enroute)_
- ✅ Geo-referenced approach plates & airport charts (ChartView / FliteCharts) _(G3X)_
- ✅ Airport surface diagrams (SafeTaxi) _(G3X)_
- ✅ Airport directory data (AOPA / AC-U-KWIK) _(G3X)_
- ✅ Checklists (user checklist files/pages) _(XCSoar, LK8000, LXNAV, G3X)_
- ❌ On-device PDF document viewer _(LXNAV)_

## Audio & voice

- ✅ Voice alerts / spoken notifications (traffic, navigation warnings) _(Enroute, G3X, LXNAV FLARM voice)_
- ⬜ AI voice assistant (offline speech control: waypoints, nav modes, radio, transponder, weather) _(Navia)_
- ❌ Spatial / 3D audio alerting over multiple speakers _(Navia)_
- ❌ SiriusXM audio entertainment _(G3X)_

## Simulation, training & demo

- ✅ On-device simulation mode (fly without GPS for training) _(XCSoar, LK8000)_
- ✅ Flight simulator integration (Condor / LXSim input for training) _(LXNAV, SeeYou Navigator Condor dongle)_
- ✅ Demo / scripted replay mode for testing and screenshots _(Enroute)_

## Cloud services, accounts & licensing

- ✳️ Cloud platform / ecosystem hub (LX Cloud, LXNAV Connect, SeeYou Cloud, WeGlide) _(Navia, LXNAV, SeeYou Navigator, WeGlide Copilot)_ (partial: no own cloud services, but integration with others, where possible)
- ⬜ Web-based configurator / desktop styler _(Navia Configurator, LXNAV LX Styler)_
- ✅ Automatic post-landing flight upload to multiple services _(LXNAV Connect, SeeYou Navigator, WeGlide Copilot)_
- ❌ Freemium / subscription / feature-license monetization (paid tiers, activation codes, trials, in-app purchase) _(WeGlide Copilot, SeeYou Navigator, XCTrack Pro, LXNAV/Navia license codes)_
- ⬜ Security & identity (device attestation, biometric login, pin/admin profile locks) _(WeGlide Copilot, LXNAV)_
- ✅ Privacy-first design (no account, no telemetry, offline-capable) _(Enroute)_

## Paragliding & hike-and-fly

- ⬜ Paragliding mode (PG tasks with ESS/conical turnpoints, PG polars/EN classes, PG contest rules) _(LK8000, XCTrack race model)_
- ⬜ Pilot profiles by experience level (Kiss/Easy/Expert/Paramotor) _(XCTrack)_
- ⬜ Hike & Fly recording with Strava upload _(SeeYou Navigator)_

## Platform & hardware integration

- ⬜ Dedicated / embedded device integration (Kobo e-readers, OpenVario, AIR³, Oudie N/Omni) _(XCSoar, LK8000, XCTrack, SeeYou Navigator)_
- ✅ Modular multi-display, multi-seat avionics architecture (compute unit + displays + hub) _(Navia, LXNAV RS485 bus, G3X dual displays)_
- ⬜ Dedicated hand controller / remote stick with haptics _(Navia Grip, LXNAV remote stick)_
- ❌ High-brightness sunlight-readable displays (FALD, ambient-light adaptation) _(Navia, G3X, LXNAV)_
- ❌ Power-over-Ethernet power + data backbone _(Navia)_
- ✅ Broad multi-platform reach from a single codebase (Android/iOS/desktop/web) _(Enroute, XCSoar, WeGlide Copilot PWA)_

## Utilities & miscellaneous

- ✅ Sun ephemeris & sunset/twilight warnings ("arrival past sunset") _(XCSoar; XCTrack sunset/civil-twilight fields)_
- ✅ Stopwatch _(LXNAV)_
- ✅ Position / ATC report page (magnetic radial + distance) _(LXNAV)_
- ❌ GPS week-rollover time fix _(XCTrack)_

# Discovery: Features

This document contains a single categorized list of all features and subfeatures
found across the analyzed systems, compiled from the discovery documents in this
directory.

Each feature is prefixed with a status marker, so we can triage which ones to
target for our app:

- ✅ target
- ✳️ maybe / partial
- ❌ skip
- ⬜ undecided

## Map display & interaction

- ✅ Vector / topographic map rendering (client-side vector, e.g. shapefile/Mapsforge/MapLibre; or pre-rendered raster tiles)
- ✳️ Raster / satellite / sectional-chart basemaps *(SeeYou Navigator, Navia, G3X, Enroute)* (partial: only for satellite/weather for now)
- ❌ Custom map render themes / styles *(XCTrack)*
- ✅ Terrain shading / elevation (DEM colouring, hillshade/slope shading, colour schemes, e-ink/greyscale rendering)
- ✳️ Map orientation (track / north / target up; plus variants: heading-up, goal-up, downwind-up, desired-track-up, separate circling orientation) (partial: just track / north / target up)
- ✅ Auto-zoom & circling zoom (mode-dependent)
- ✅ Smart / offset pilot position on screen *(XCTrack, XCSoar)*
- ✅ Manual pan & zoom (gestures, buttons, knobs, dedicated pan mode)
- ✅ Snail trail / flight trail (length modes, colouring by vario/altitude/speed, wind-drift compensation, engine-on colouring)
- ✅ Glide range ("reachability") line / area (terrain-aware footprint, turning reach, safety-MC based fill, glide circles)
- ✅ Thermal markers on map (own-climb thermal history, drift-compensated sources)
- ✅ Thermal hotspot overlays (historical/crowd-sourced, e.g. thermal.kk7.ch, WeGlide live thermals) *(SeeYou Navigator, XCTrack, WeGlide Copilot)*
- ✅ Markers / pilot events on map (manual & automatic markers, PEV)
- ✅ Waypoint & landable symbology / labels (icon sets, reachability colouring, runway direction, declutter, arrival-height labels)
- ✅ "What's here" query (multi-object tap/pan query with details)
- ⬜ Multimaps / split-screen map views (map + sideview / terrain / airspace) *(LK8000)*
- ⬜ Visual Glide: horizontal strip of glide bars to nearby landables *(LK8000)*
- ⬜ 3D synthetic map view (terrain with rivers/roads/airspace/traffic traces) *(LXNAV, G3X Synthetic Vision)*

## Waypoints & navigation

- ✅ Waypoint database (multiple simultaneous files, unlimited points, search)
- ✅ Landable vs non-landable distinction (incl. outlanding-field layers with quality grading)
- ✅ Nearest (waypoint / landable / airfield) lists and sorting
- ✅ Go-to / direct navigation
- ✅ Waypoint details (info text, images, runway, frequency, elevation)
- ✅ Alternates / safety-landing selection (alternates dialog, abort mode, best-alternate)
- ✅ Multi-target / secondary-target navigation ("X>", alternate arrival readouts) *(LK8000)*
- ⬜ Oracle-style "what's useful near me" assistant *(LK8000)*
- ⬜ Approach assistant / runway selection for landables *(LK8000)*
- ⬜ 3D route planning around terrain and airspace *(XCSoar)*
- ✅ GA flight-route editor (leg-based nav routes as distinct from scored tasks) *(Enroute, G3X flight plans)*

## Cross-country tasks

- ✅ Racing tasks (start arm/gates, start speed/height limits, PEV start)
- ✅ Assign Area Tasks (AAT / MAT / TAT; isolines, target moving, min task time)
- ✅ Cross-country task types (FAI triangle, out-and-return, DMSt, Grand Prix, national variants)
- ✅ Observation zones & start/finish/turnpoint rules (cylinders, FAI sectors, keyholes, lines, ESS/goal semantics, per-point overrides)
- ✅ Task manager (build / edit; list and map editing; templates)
- ✅ Task calculator (in-flight required speeds, AAT range/what-if, time gates)
- ✅ In-flight task edit
- ✅ FAI badge / record support (badge/record rules, finish-below-start handling)
- ✅ FAI triangle assistant (live triangle-closing guidance) *(LXNAV, SeeYou Navigator, XCTrack, LK8000 FAI Assistant)*
- ✅ Task import / export / declaration to logger (file formats, declaration to external loggers/internal recorder)
- ✅ Task download from online sources (SoaringSpot, WeGlide, cloud collections)
- ✅ Task sharing via QR code / NFC / email / messaging *(XCTrack, SeeYou Navigator)*

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
- ⬜ Inertial/aerodynamic air-data models (HAWK: inertial wind, netto, AoA, sideslip) *(LXNAV; Navia "inertial vario/wind")*
- ✅ Wind estimation (circling / zigzag / compass / combination / external; layered wind statistics)
- ✅ Wind display & manual override (vectors, per-layer editing, freehand draw)
- ✅ Thermal locator / assistant / centering aid (graphical + audible)
- ✅ Thermal profile (thermal band / climb-vs-altitude graphs)
- ⬜ Convection forecast (cloud base / convection ceiling from temp trace or forecast products)
- ✅ Pressure-altitude / cabin-altitude tool *(Enroute)*
- ✅ Density-altitude computation *(Enroute, G3X)*
- ✅ Potential-temperature trigger aid *(LXNAV)*
- ✅ Physiological sensors: heart rate, SpO₂ *(LXNAV, LK8000, G3X)*

## Weather

- ✅ METAR / TAF (decoded display, map flags, QNH extraction, meteograms)
- ✅ Forecast overlays (SkySight / TopMeteo / RASP / pc_met; opacity, forecast-time animation)
- ✅ Wind aloft / weather-station data (multi-level forecast wind, live station wind, FANET weather beacons)
- ✅ In-flight weather updates (live refresh over connectivity, cached weather after signal loss)
- ✅ Rain radar overlay *(WeGlide Copilot, XCTrack, SeeYou Navigator, LXNAV)*
- ✅ Live weather-satellite imagery (with time slider/animation) *(WeGlide Copilot, LXNAV, SeeYou Navigator)*
- ✅ Datalink weather (SiriusXM / FIS-B: NEXRAD, echo tops, lightning, PIREPs, AIRMET/SIGMET, TFRs) *(G3X)*

## Airspace

- ✅ Airspace display (classes, per-class styling/filtering, altitude filters, inactive/periodic zones)
- ✅ Proximity / incursion warnings + acknowledgement (predicted incursion, graded warnings, per-zone/group dismiss with duration)
- ✅ Airspace query / details (vicinity lists, details dialogs, per-zone enable/disable, in-app editing)
- ✅ NOTAM handling (live NOTAM download as airspace, filters, details; regional TFR/DABS products)
- ❌ Post-flight airspace-crossing scan/review *(WeGlide Copilot)*

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
- ✅ ADS-B In traffic (via PowerFLARM/Stratux/dual receivers or TIS/TAS) *(XCSoar, LXNAV, Navia, G3X)*
- ⬜ TIS-A / TAS / TCAS traffic with TCAS symbology and voice advisories *(G3X)*
- ⬜ PCAS non-directional target display *(LXNAV)*
- ⬜ FANET support (traffic, weather beacons, device naming) *(LK8000, XCTrack, SeeYou Navigator/Oudie Fanet+)*
- ⬜ Electronic conspicuity integration (SafeSky; EC-ID pairing across networks) *(XCTrack)*

## Avionics & airframe

- ✅ Battery / voltage monitoring (internal + external batteries, battery-type profiles, per-cell monitors)
- ✅ GPS status / connection / altitude source (satellite views, multi-source failover, geoid correction, baro-source arbitration)
- ✅ Engine / powered flight (ENL, MoP, engine hours, fuel model, e-Glide energy reference)
- ⬜ Full piston-engine indication (EIS: RPM, fuel flow/calculator, CHT/EGT, Lean Assist, CAS messages) *(G3X; Navia EMU/MOP devices)*
- ✅ Microphone-based ENL recording into IGC *(WeGlide Copilot)*
- ✅ Radio frequency management / control (active/standby, dual watch, tune from waypoint, driver-based control)
- ✅ Transponder (XPDR) control / squawk (code/mode/ident via supported drivers)
- ✅ Switch inputs (gear / flap warnings, configurable digital inputs, AGL-triggered reminders)
- ✅ Multiple external devices / slave mode (device slots, value merging, NMEA pass-through, repeater displays)
- ✅ Two-seat / dual-cockpit operation (front/rear sync of Mc/ballast/target/wind) *(LXNAV, Navia dual displays & grips)*
- ✅ Weight & balance / CG-envelope calculator *(LXNAV, G3X)*
- ⬜ Electrical-system / electronic circuit-breaker integration (Vertical Power) *(G3X)*
- ✅ Carbon-monoxide detector integration *(G3X)*
- ❌ Uninterruptible / backup power management *(Navia UPS)*
- ✅ Analog & telemetry inputs (multi-channel analog acquisition, engine/battery telemetry) *(LXNAV LXDAQ/JRES)*

## Data fields (InfoBox / navbox / widget system)

- ✅ Configurable data-field grid (selectable geometries, free widget placement, styling)
- ✅ Multiple data-field pages / layouts (per-orientation layouts, import/export, community layout sharing)
- ✅ Per-flight-mode auto layout (mode-driven InfoBox sets, auto thermal page)
- ✅ Altitude / altimetry values (QNH, pressure altitude, FL, AGL, above-takeoff)
- ✅ Speed values (GS / TAS / IAS / optimal / XC speed)
- ✅ Direction values (track / bearing / heading / radial / steering course)
- ✅ Time values (UTC / local / ETA / ETE / sunset / twilight / flight time)
- ✅ Touch / gesture interaction with data fields (tap-to-edit, long-press editing, interactive buttons)
- ✅ Bottom bar with multi-mode nav-box stripes *(LK8000)*
- ⬜ Action widgets (camera, phone call, brightness, vario mute, zoom, page/waypoint switching) *(XCTrack)*
- ❌ Web-page widget (live web content with position placeholders) *(XCTrack)*

## Analysis & review

- ✅ Barograph / altitude trace (incl. working-band estimation)
- ✅ Climb history / thermal analysis (per-climb charts, thermal graphs, leg statistics)
- ✅ Wind analysis (wind vs altitude)
- ✅ Glide polar analysis
- ✅ Task analysis (per-leg statistics, AAT rendering)
- ✅ OLC / contest analysis (optimal path display, live score estimation)
- ✅ Airspace cross-section / side view (terrain + airspace profile ahead)
- ✅ Vario histogram and temperature-trace pages *(XCSoar)*
- ⬜ Post-landing statistics report *(SeeYou Navigator, WeGlide Copilot)*
- ⬜ Pilot logbook (auto-written, totals, photos/videos, manual entries) *(SeeYou Navigator)*
- ❌ CSV flight/engine data logging for off-board analysis *(G3X)*

## Contest optimization (live)

- ✅ Live contest optimization in-flight (OLC / WeGlide / XContest / DMSt / FAI rule sets)
- ⬜ Flight trace maintenance (continuous optimized-trace computation)
- ✅ Live scoring / achieved distance (score/distance data fields, live ranking)
- ✅ One-tap post-flight contest upload (OLC, WeGlide, XContest, DHV-XC) *(SeeYou Navigator, WeGlide Copilot, XCTrack auto-claim)*

## Live tracking

- ✅ Live tracking upload (OGN / SkyLines / LiveTrack24 / XContest / cloud services / FFVL / PureTrack / Traccar)
- ⬜ Retrieve / crew comms / position sharing (team position sharing, share-to-external-apps, live view of other pilots)
- ⬜ In-app pilot-to-pilot messaging over livetracking *(XCTrack)*
- ⬜ Automatic position-sharing on takeoff / auto flight claim after landing *(XCTrack)*
- ⬜ Location sharing to external apps (`geo:` URLs, share sheets) *(Enroute)*

## Instrument & device connectivity (I/O)

- ✅ Bluetooth (classic / SPP)
- ✅ BLE (incl. sensor service discovery)
- ✅ Serial (RS-232 / TTY / RS485 device bus)
- ✅ USB (USB-serial, memory-stick transfer)
- ✅ TCP/IP / network (TCP client/server, UDP, Wi-Fi modules)
- ❌ Built-in cellular connectivity (global SIM / LTE) *(Navia)*
- ✅ NMEA input parsing (position, air data, traffic; GDL90, X-GPS and vendor sentences)
- ✅ NMEA / data output (drive other devices, slave displays, custom output sentences)
- ✅ Positioning source selection (internal / external, priority ordering, automatic fallback)
- ✅ Vendor protocols (LXNAV, FLARM, CAI, Vega, Flymaster, XCVario, radio/XPDR bridges, and many more)
- ✅ Device management dialogs (device memory/setup management, e.g. CAI302, Vega) *(XCSoar)*
- ✅ IGC file download from connected devices (FLARM, LX, EOS/ERA) *(LK8000)*
- ✅ Terminal / COM-port monitor for I/O debugging *(LK8000)*
- ⬜ AR smart-glasses output (ActiveLook HUD) *(XCTrack)*
- ❌ Open hardware / documented integration protocols *(Navia)*

## Data & file management

- ✅ Waypoint files (CUP, CUPX, DAT, WPT/OZI, CompeGPS, OpenAIP, GPX, GeoJSON)
- ✅ Airspace files (OpenAir, CUB, SUA, OpenAIP; server-sourced databases)
- ✅ Terrain / topology / map data (map packages, MBTiles, DEM, scanned/raster charts)
- ✅ In-app data download & updates (region managers, repository manifests, web airspace auto-update)
- ✅ Aircraft / plane profiles (polar, ballast, registration, competition ID, FLARM ID, weight & balance)
- ✅ Handicap / polar lists (built-in polar stores, handicap factors)
- ⬜ GA route file interchange (GPX, FPL, PLN, TripKit) *(Enroute, G3X SD-card flight plans)*
- ✅ Cloud data sync (waypoints, tasks, maps, profiles, settings across devices) *(SeeYou Navigator, LXNAV Connect, LX Cloud/Navia)*
- ✅ Configuration sharing via files / QR codes / community libraries *(XCTrack)*

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
- ❌ Scripting / automation (embedded Lua engine; event-driven "automatic actions") *(XCSoar Lua, XCTrack automatic actions)*
- ✅ Status dialogues (flight / times / system / rules status views) *(XCSoar)*

## Flight instruments & EFIS

- ✅ Attitude indicator / artificial horizon (AHRS-driven) *(XCSoar, LXNAV, Navia, G3X)*
- ✅ Primary Flight Display (airspeed/altitude tapes, HSI, slip/skid) *(G3X, LXNAV PFD symbols)*
- ✅ Synthetic Vision (3D forward-looking terrain/obstacle/traffic/runway) *(G3X, LXNAV 3D view)*
- ⬜ Angle of Attack indication with aural alerting *(G3X, LXNAV HAWK)*
- ⬜ G-meter *(LXNAV)*
- ✅ Flap tape with suggested flap position *(LXNAV)*
- ❌ Autopilot / Automatic Flight Control System (flight director, lateral/vertical modes, coupled approaches) *(G3X)*
- ❌ Electronic Stability & Protection (envelope protection) *(G3X)*
- ✅ VNAV vertical navigation to altitude constraints *(G3X)*
- ⬜ IFR GPS navigation (airways, holds, procedures via external navigator) *(G3X)*

## Terrain & obstacle safety

- ⬜ Terrain proximity alerting (colour-coded terrain, terrain warnings) *(G3X, SeeYou Navigator terrain warnings)*
- ✅ Obstacle databases & warnings (worldwide or regional cable/obstacle data, hazard display modes) *(SeeYou Navigator, XCTrack, G3X, LXNAV FLARM obstacle warnings)*

## Charts & documents

- ✅ Visual approach charts / georeferenced chart overlays (VAC, TripKit) *(Enroute)*
- ✅ Geo-referenced approach plates & airport charts (ChartView / FliteCharts) *(G3X)*
- ✅ Airport surface diagrams (SafeTaxi) *(G3X)*
- ✅ Airport directory data (AOPA / AC-U-KWIK) *(G3X)*
- ✅ Checklists (user checklist files/pages) *(XCSoar, LK8000, LXNAV, G3X)*
- ❌ On-device PDF document viewer *(LXNAV)*

## Audio & voice

- ✅ Voice alerts / spoken notifications (traffic, navigation warnings) *(Enroute, G3X, LXNAV FLARM voice)*
- ⬜ AI voice assistant (offline speech control: waypoints, nav modes, radio, transponder, weather) *(Navia)*
- ❌ Spatial / 3D audio alerting over multiple speakers *(Navia)*
- ❌ SiriusXM audio entertainment *(G3X)*

## Simulation, training & demo

- ✅ On-device simulation mode (fly without GPS for training) *(XCSoar, LK8000)*
- ✅ Flight simulator integration (Condor / LXSim input for training) *(LXNAV, SeeYou Navigator Condor dongle)*
- ✅ Demo / scripted replay mode for testing and screenshots *(Enroute)*

## Cloud services, accounts & licensing

- ✳️ Cloud platform / ecosystem hub (LX Cloud, LXNAV Connect, SeeYou Cloud, WeGlide) *(Navia, LXNAV, SeeYou Navigator, WeGlide Copilot)* (partial: no own cloud services, but integration with others, where possible)
- ⬜ Web-based configurator / desktop styler *(Navia Configurator, LXNAV LX Styler)*
- ✅ Automatic post-landing flight upload to multiple services *(LXNAV Connect, SeeYou Navigator, WeGlide Copilot)*
- ❌ Freemium / subscription / feature-license monetization (paid tiers, activation codes, trials, in-app purchase) *(WeGlide Copilot, SeeYou Navigator, XCTrack Pro, LXNAV/Navia license codes)*
- ⬜ Security & identity (device attestation, biometric login, pin/admin profile locks) *(WeGlide Copilot, LXNAV)*
- ✅ Privacy-first design (no account, no telemetry, offline-capable) *(Enroute)*

## Paragliding & hike-and-fly

- ⬜ Paragliding mode (PG tasks with ESS/conical turnpoints, PG polars/EN classes, PG contest rules) *(LK8000, XCTrack race model)*
- ⬜ Pilot profiles by experience level (Kiss/Easy/Expert/Paramotor) *(XCTrack)*
- ⬜ Hike & Fly recording with Strava upload *(SeeYou Navigator)*

## Platform & hardware integration

- ⬜ Dedicated / embedded device integration (Kobo e-readers, OpenVario, AIR³, Oudie N/Omni) *(XCSoar, LK8000, XCTrack, SeeYou Navigator)*
- ✅ Modular multi-display, multi-seat avionics architecture (compute unit + displays + hub) *(Navia, LXNAV RS485 bus, G3X dual displays)*
- ⬜ Dedicated hand controller / remote stick with haptics *(Navia Grip, LXNAV remote stick)*
- ❌ High-brightness sunlight-readable displays (FALD, ambient-light adaptation) *(Navia, G3X, LXNAV)*
- ❌ Power-over-Ethernet power + data backbone *(Navia)*
- ✅ Broad multi-platform reach from a single codebase (Android/iOS/desktop/web) *(Enroute, XCSoar, WeGlide Copilot PWA)*

## Utilities & miscellaneous

- ✅ Sun ephemeris & sunset/twilight warnings ("arrival past sunset") *(XCSoar; XCTrack sunset/civil-twilight fields)*
- ✅ Stopwatch *(LXNAV)*
- ✅ Position / ATC report page (magnetic radial + distance) *(LXNAV)*
- ❌ GPS week-rollover time fix *(XCTrack)*

# Discovery: SeeYou Navigator

## System overview

- **Version(s) examined:** 3.9.x (App Store build 3.9.5; the "Navigator 3.0" redesign line). Same codebase powers the Oudie N and Omni hardware.
- **Date(s) examined:** 2026-07-06 — via naviter.com, the Apple App Store / Google Play listings, and the Naviter Knowledge Base (kb.naviter.com), which is current to Jan–Feb 2026.
- **Platform(s):** iOS (16.6+) and Android phones/tablets. The identical app is the OS of Naviter's Oudie N and Omni (Android-based) dedicated devices. No PC/Mac/Linux build (that role is filled by the separate *SeeYou for PC* and *SeeYou.Cloud* products in the same subscription bundle).
- **License:** Proprietary / commercial. Part of the *SeeYou Subscription* (~$9.99/month or $54.99/year). A free tier exists with reduced functionality; several features (waypoint/task collections, logbook, rain radar, sync) expect a SeeYou Cloud account.
- **Offline behavior:** IGC recording and core navigation work fully offline. Offline data can be pre-downloaded per region: base maps, topographic maps, satellite imagery, terrain, airspace, and waypoints. Live features (OGN traffic, rain radar, SkySight/TopMeteo overlays, live wind, cloud sync, contest upload, Soaring Spot task load) require connectivity. Maps are pre-rendered **raster tiles**, so offline coverage is by downloaded region rather than global vector data.
- **Configuration model:** Deliberately simplified vs. XCSoar/LX. Per-aircraft profiles ("Wing / Glider hangar": polar, wing area, weight, registration, competition ID, FLARM ID). Multiple screen **layouts** (separate portrait/landscape, importable/exportable, per-flight-phase). Settings sync to a new device on first sign-in. Drag-and-drop navbox editing via long-press.

## Feature inventory

**Status legend:**
- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

### Map display & interaction
- `◐` Vector / topographic map rendering: Maps are **raster tile** sets (Light, Terrain, Outdoor, Satellite), not client-side vector maps like XCSoar's CIT. "Topographic" detail is present as pre-rendered Outdoor/Terrain schemes.
- `●` Terrain shading / elevation: Terrain and Outdoor schemes give shaded relief with high contrast; elevation data drives AGL and obstacle warnings.
- `●` Map orientation (track / north / target up): Compass indicator toggles orientation modes.
- `●` Auto-zoom & circling zoom (mode-dependent): Dedicated Auto Zoom function plus thermal-assistant circling zoom that zooms in on circling and back out on exit.
- `●` Manual pan & zoom: Plus a "swipe to zoom" gesture and zoom buttons.
- `●` Snail trail / flight trail: Shown on map; color-coded by vario in circling.
- `◐` Glide range ("reachability") line: Reachability is expressed as green/red color-coding of landables plus arrival-altitude/required-L/D labels (shown in-flight only). No evidence of a continuous glide-range polygon/amoeba line.
- `●` Thermal markers on map: Thermal-assistant "bubbles" (drift downwind, sized by strength) and the KK7 Thermals overlay layer.
- `●` Markers / pilot events on map: "Mark event" button; PEV markers.
- `◐` Waypoint & landable symbology / labels: Airports and paragliding take-off/landing layers with labels; airfield detail panels show frequency and runway. Depth of custom symbology config is limited.
- `◐` "What's here" query: Long-press any point on the map to inspect / GoTo; not a rich multi-object "what's here" list.

### Waypoints & navigation
- `●` Waypoint database: SeeYou Cloud collections sync seamlessly; CUP import; SeeYou airport/landing database as layers.
- `●` Landable vs non-landable distinction: Airfields and landing sites are distinct target types with reachability coloring.
- `◐` Nearest (waypoint / landable / airfield): Reachability coloring and "Nearest Landing" / "Nearest Airspace" navboxes surface nearest landables; a full ranked nearest-list dialog is less prominent than in XCSoar/LX.
- `●` Go-to / direct navigation: Tap target → GO TO; red dotted course line.
- `◐` Waypoint details (info, image, runway, freq): Details panel shows coordinates, altitude, distance/bearing, arrival altitude, comm frequency, runway. Waypoint photos not confirmed.
- `◐` Alternates / safety-landing selection: Navigate-back-to-Takeoff and Last-thermal targets, plus landable reachability; no dedicated multi-alternate manager observed.

### Cross-country tasks
- `●` Racing tasks: First-class task type.
- `●` Assign Area Tasks (AAT): First-class task type.
- `◐` Cross-country task types (FAI triangle, out-and-return, DMSt, etc.): Task types are XC / Racing / AAT; an **FAI Triangle Assistant** (rotatable FAI area) aids triangle shaping. Named preset templates (out-return, DMSt) not exposed as such.
- `◐` Observation zones & start/finish/turnpoint rules: OZ shape and size are selectable per turnpoint, including custom (non-cylindrical) zones; second point defaults to start, last to ESS. Full competition rule-set editing is lighter than dedicated comp systems.
- `●` Task manager (build / edit): Manual turnpoint entry, add/edit/remove/reorder, task-property editor.
- `◐` Task calculator (in-flight required speeds etc.): Task navboxes (Task Speed, Task Required Speed, Speed to Gate, Time to Gate, Task Delta Time, Task % Flown, task ETA/ETE) provide the numbers; less of an interactive "what-if" calculator than SeeYou desktop.
- `●` In-flight task edit: Tasks can be modified after declaration; A/B task swap via recent-tasks list.
- `◐` FAI badge / record support: IGC logging + FAI Triangle Assistant support the flying; badge/record validity depends on an approved logger (Oudie N/Omni carry IGC approval; the phone app does not).
- `●` Task import / export / declaration to logger: Load from Cloud collection, QR code, Soaring Spot; declare task to **LXNAV** (v9.52+) and **FLARM** instruments.

### Glide computer
- `●` Flight modes + auto display switching (cruise/circling/final): Flight-phase indicator (waiting/climbing/cruise/final glide); thermal assistant auto-switches the map on circling.
- `●` MacCready setting (manual): MacCready navbox + up/down buttons.
- `○` Auto MacCready (modes): No auto/Block/dolphin MacCready mode found.
- `◐` Glide polar (library + custom): Polar comes from the aircraft database entry (manufacturer/model, wing area, weight). Free-form custom polar coefficient entry not documented.
- `◐` Ballast / bugs / weight setup: Ballast is adjustable (in-flight ballast slider); weight is part of the aircraft profile. A separate "bugs"/dirt factor was not found.
- `◐` Speed to fly / speed command / risk factor: "Speed to Fly" navbox present (visual). No audio speed-director or risk/altitude-factor setting confirmed.
- `◐` Safety heights / safety MacCready: Single "Safety Height" reserve (typ. 300–500 m) added to all arrival-altitude calcs and used for terrain warnings. No separate safety-MacCready.
- `●` Final glide calculator (wind, altitude required): Continuous arrival altitude + required L/D to target, wind-corrected.
- `●` Task speed estimation: Task Speed / Required Speed / XC Speed navboxes.
- `?` Optimal cruise track: A "TO (angle)" navbox exists; explicit optimal-track-to-next-TP guidance not confirmed.

### Atmosphere & instruments
- `●` Variometer display: Round and linear vario indicators + Vario navbox.
- `●` Average climb: AVG Vario, Average Vario 60′, Last Thermal navboxes.
- `●` Audio variometer: Configurable audio vario — sensitivity, averager, beep-on-ground, sink alarm, custom tone; "thermal sniffer" tone.
- `◐` Air-data inputs (external sensors): Pressure/airspeed via BLE varios (Flytec Sensbox, XCVario, LXNAV, etc.) using $LXWP0/LK8EX/PGRMZ/XCTRACER. Depends on external hardware; phone alone provides GPS + barometric altitude only.
- `◐` Wind estimation (circling / zigzag / compass / external): Wind is auto-calculated from flight data; external/"Live Wind" from weather stations. Specific algorithm choice (circling vs zigzag) not exposed.
- `●` Wind display & manual override: Wind, Wind Direction (+letters) navboxes; Live Wind map layer.
- `●` Thermal locator / assistant / centering aid: Thermal assistant with Zoom-only / Bubbles modes + "ding" recentering action sound.
- `◐` Thermal profile: Vario-colored track + drifting bubbles visualize structure; no altitude-vs-climb-rate profile graph.
- `◐` Convection forecast (cloud base etc.): Via SkySight/TopMeteo overlays (Thermal tops, PFD, Convergence, Waves) — requires a separate third-party subscription. No standalone computed cloudbase value.

### Weather
- `○` METAR / TAF: Not found.
- `●` Forecast overlays (SkySight / TopMeteo / RASP): SkySight (Satellite, Wind, PFD, Thermal tops, Convergence, Waves) and TopMeteo overlays; third-party subscription required.
- `◐` Wind aloft / weather-station data: Live Wind layer from weather stations; wind-aloft as such comes through the forecast overlays.
- `●` In-flight weather updates: Rain Radar live layer and live overlays update in flight (with connectivity).

### Airspace
- `●` Airspace display (classes, filtering): Classes A–G plus zone types, per-class visibility toggles, "always visible" classes, colour/line-style coding, periodically-active zones in yellow.
- `●` Proximity / incursion warnings + acknowledgement: Visual + audio warnings; zones can be temporarily hidden with a hide-duration (acknowledgement); "hide above altitude" filter.
- `◐` Airspace query / details: Zones are inspectable on the map; depth of a dedicated details/description dialog is limited.
- `◐` NOTAM handling: No general NOTAM feed; region-specific: real-time **TFR** updates (USA) and **DABS** daily airspace bulletin (Switzerland).

### Traffic & collision awareness (FLARM)
- `●` FLARM Traffic on map: Via connected FLARM/PowerFLARM (PFLAA/PFLAU) or integrated Flarm Horizon XC (Fanet+) on Oudie N/Omni.
- `●` OGN Traffic on map: Live OGN layer relays distant Flarm targets over the network.
- `●` FLARM and OGN Traffic deduplication: Manual/auto Flarm-ID entry in the aircraft profile prevents the pilot's own glider being double-reported.
- `◐` FLARM radar view: Traffic shown on the moving map with altitude/climb; a dedicated separate "radar" panel like classic SeeYou Mobile was not confirmed in the current app.
- `?` FLARM traffic dead reckoning: Not documented.
- `◐` Collision / proximity warnings: Traffic (incl. altitude and climb) is displayed and labels auto-declutter to strongest climbers; explicit FLARM collision-alert semantics (level/threat) not clearly documented for the app itself.
- `●` Traffic details dialogue: Tap a target for info; edit/star buddies.
- `◐` Traffic ID / registration lookup: Buddy names editable; OGN/registration linkage via DDB. Automatic reg lookup for arbitrary traffic not confirmed.
- `●` Team flying / buddy codes: Buddy starring/highlighting, navigate-to-buddy, plus gliding **Team Code** (share position via short code).

### Avionics & airframe
- `◐` Battery / voltage monitoring: Device battery-level indicator. External glider-bus voltage monitoring not confirmed.
- `●` GPS status / connection / altitude source: GPS signal + connection indicators; GPS Altitude navboxes; auto-selects external device position when connected.
- `○` Engine / powered flight (ENL, MoP, engine hours): No ENL/engine navboxes; powered-sailplane IGC approval is a hardware-recorder property, not the phone app.
- `○` Radio frequency management / control: Airfield COM frequency is displayed, but no radio control.
- `○` Transponder (XPDR) control / squawk: Not present.
- `○` Switch inputs (gear / flap warnings etc.): Not present.
- `◐` Multiple external devices / slave mode: Phone can pair to Oudie N/Omni ("Connect your phone to Oudie N/Omni"); primary model is a single BLE data source rather than many simultaneous devices.

### Data fields (InfoBox system)
- `●` Configurable data-field grid: "Navboxes" — freely moved, resized, styled (colour/transparency) via long-press edit mode.
- `●` Multiple data-field pages / layouts: Multiple custom layout pages; one-tap switching; import/export; separate portrait/landscape.
- `◐` Per-flight-mode auto layout: Layouts can be prepared per flight phase and switched; thermal-assistant changes the map automatically, but fully automatic per-mode navbox swapping is not clearly automatic.
- `●` Altitude / altimetry values (QNH, pressure alt, FL, AGL): Altitude (MSL), AGL, Flight Level, GPS Altitude, Above Takeoff, Ground.
- `●` Speed values (GS / TAS / IAS / optimal): Ground Speed, True Airspeed, Indicated Airspeed, Speed to Fly.
- `●` Direction values (track / bearing / heading): Track (+magnetic, +letters), Bearing (+magnetic), Radial, TO angle.
- `●` Time values (UTC / local / ETA / ETE / sunset / flight time): Time, Flight Time, Target ETA/ETE, Task ETA/ETE, Time-to-Go/Gate. (Sunset value not explicitly listed.)
- `●` Touch / gesture interaction with data fields: Long-press to add/move/resize/edit/remove; interactive MacCready and turnpoint buttons; swipe gestures.

### Analysis & review
- `◐` Barograph / altitude trace: Post-landing statistics report + Altitude 2h/2.5h navboxes; full barograph analysis lives in SeeYou Cloud/PC, not the app.
- `◐` Climb history / thermal analysis: Last Thermal, Average Vario 60′, Altitude Gain navboxes in-flight; deep analysis in SeeYou.
- `◐` Wind analysis: Live wind + wind navboxes in-flight; retrospective analysis in SeeYou.
- `○` Glide polar analysis: Not in the app (SeeYou desktop feature).
- `◐` Task analysis: Task statistics report after landing; detailed task analysis in SeeYou.
- `◐` OLC / contest analysis: Live OLC/triangle optimization navboxes (see 3.12); post-flight OLC analysis via SeeYou Cloud upload.
- `○` Airspace cross-section: Not found in the app.

### Contest / WeGlide optimization (live)
- `◐` Live WeGlide optimization in-flight: Live OLC-style optimization navboxes present (Triangle, Closed OLC, 3 TP, Closed 3 TP, Flown Distance); WeGlide-specific live scoring not separately confirmed (WeGlide is supported for post-flight upload).
- `●` Flight trace maintenance: Continuous IGC trace + on-map trail.
- `●` Live scoring / achieved distance: Flown Distance, Triangle, Closed OLC / 3 TP navboxes update in flight.

### Live tracking
- `●` Live tracking upload (OGN / SkyLines / cloud): OGN live tracking (position relayed while receiving); SeeYou Cloud integration. SkyLines not confirmed.
- `◐` Retrieve / crew comms / position sharing: Team Code + buddy tracking + OGN (family/search-and-rescue) provide position sharing; no dedicated retrieve-crew messaging.

### Instrument & device connectivity (I/O)
- `◐` Bluetooth: Naviter Bluetooth dongles bridge FLARM/LXNAV/LX-Navigation to the phone; app connectivity is BLE-centric (classic SPP not emphasized, esp. on iOS).
- `●` BLE: Primary connection method (Bluetooth Low Energy) to a broad device list (XCTracer, Flymaster, Flytec Sensbox, LXNAV S10/Flarm Mouse, PowerFLARM Flex/Fusion, XCVario, Syride, SkyDrop, etc.).
- `○` Serial: No direct serial from the phone app (only via dongles/hardware).
- `○` USB: Not used by the app.
- `◐` TCP/IP / network: Network is used for OGN, weather, cloud, Soaring Spot. Device NMEA-over-TCP input not documented.
- `●` NMEA input parsing: $GPGGA, $GPRMC (position); $LXWP0, XCTRACER, LK8EX, PGRMZ (pressure); PFLAA, PFLAU (traffic).
- `◐` NMEA / data output (drive other devices): Task declaration to LXNAV/FLARM and PEV-marker output; general NMEA-out to drive third-party displays not documented.
- `●` Positioning source selection (internal / external): Uses external device GPS/pressure when connected (preferred for accuracy), phone sensors otherwise.
- `●` Vendor protocols (LXNAV / FLARM / …): LXNAV, FLARM/PowerFLARM, LX Navigation, XCVario, Flymaster, Flytec, etc.

### Data & file management
- `●` Waypoint files (CUP / …): CUP import + SeeYou Cloud collections; CUP/CUPX specs are Naviter's own.
- `●` Airspace files (OpenAir / CUB / …): Custom OpenAir (.txt) and SeeYou (.cub) files, or the auto-updated SeeYou database.
- `●` Terrain / topology / map data: Downloadable per region — base, topographic, satellite, terrain (raster tiles).
- `●` In-app data download & updates: Offline Data > Regions; auto-updating SeeYou database; app/home update prompts on Oudie N/Omni.
- `●` Aircraft / plane profiles (polar, ballast, reg, comp ID): Glider hangar with polar, wing area, weight, registration, competition ID, FLARM ID; applied to calcs, logger, and FLARM/FANET.
- `◐` Handicap / polar lists: Polars come from the model database; competition handicap lists (DAeC/IGC) are a SeeYou-desktop concept, not confirmed in the app.

### Logging & recording
- `●` IGC flight log recording: Automatic takeoff/landing detection, background recording, IGC output; manual H&F mode for hike-and-fly.
- `◐` Approved / signed logger (badge / record): Oudie N/Omni hardware carries IGC approval (and CIVL approval for Omni-created flights); the phone app produces IGC but is not a security-signed logger.
- `○` Flight replay: Not in the app (SeeYou Cloud/PC feature).
- `●` Pilot events / markers logging: Mark-event button; PEV markers logged and sent to connected devices.

### Configuration & UI
- `●` Configuration profiles (per user / per aircraft / global): Per-aircraft profiles + multiple layouts + account-based settings sync to new devices.
- `●` Screen layout / data-field geometry: Full drag/resize/style of navboxes; multiple pages; import/export.
- `◐` Day / night / high-contrast modes: "Light" map for bright-sun contrast; brightness via OS. Explicit auto day/night theme not confirmed.
- `●` Units: Configurable (metric/imperial/FL, etc.) in System settings.
- `●` Input / gestures / hardware buttons: Long-press, swipe-to-zoom, gesture zoom, on-screen buttons; hardware buttons on Oudie N/Omni.
- `●` Language / localization: English, German, French, Italian, Spanish, Polish, plus Finnish and others; more added on request.

### Additional features (outside the shared taxonomy)

- `●` Logbook: Auto-writing SeeYou logbook with photos/videos, manual flight entry, totals (this year / last 12–24 months / overall), post-landing statistics report.
- `●` One-tap contest upload: Direct upload to OLC, WeGlide, XContest, DHV-XC right after landing.
- `●` Hike & Fly + Strava: H&F recording of the walking segment; upload activity to Strava.
- `●` Ecosystem sync: Seamless waypoint/task/flight sync across SeeYou (PC), SeeYou.Cloud, and Oudie/LX 9000 devices.
- `●` KK7 overlays: Skyways and Thermals overlays from thermal.kk7.ch.
- `●` World-wide obstacle database: Global obstacle warnings (enable/disable, DB statistics).
- `●` Live Satellite imagery: From SkySight/TopMeteo (with their subscription).
- `◐` Condor simulator support: Naviter Condor Dongle streams Condor flight data into the app for practice/instruction.

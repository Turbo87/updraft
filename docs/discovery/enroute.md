# Discovery: Enroute Flight Navigation

> **Framing note.** Enroute Flight Navigation (by Akaflieg Freiburg) is a **VFR
> moving-map navigation app for powered GA / touring aircraft**, not a soaring
> flight computer. It deliberately has no glide computer, no MacCready/polar
> model, no thermal assistant, and no cross-country task/scoring engine. As a
> result, large parts of the soaring-oriented taxonomy below come back `absent`.
> Where Enroute has a *different but adjacent* capability (e.g. a flight-route
> editor instead of a task manager, a side-view instead of an airspace
> cross-section analysis), that is called out in the notes.

## System overview

- **Version(s) examined:** 3.3.3 (git `main`, `CMakeLists.txt` `project(enroute VERSION 3.3.3)`)
- **Date(s) examined:** 2026-07-06 (source), app release 3.3.3 dated 2026-06-08
- **Platform(s):** Android & iOS (primary), plus Linux (Flatpak/desktop), macOS, Windows — one Qt/QML/C++20 codebase with per-platform adaptors (`src/platform/*_{Android,iOS,Linux,MacOS,Windows}.cpp`)
- **License:** GPL-3.0 (application code). Aeronautical map/data are separate and not GPL.
- **Offline behavior:** Fully usable offline once maps are downloaded — moving map, airspaces, airfields/navaids, waypoint & VAC libraries, route planning, and live navigation all work with no connectivity. Network is needed only for: map/data download & near-weekly updates, METAR/TAF, NOTAMs, and internet-based OGN traffic.
- **Configuration model:** Simple. Global `QSettings` (`GlobalSettings`) plus three user libraries — Aircraft library (multiple aircraft profiles), Flight-Route library, and Waypoint library. No per-user profile system, no scripting, no configurable screen layout.

## Feature inventory

**Status legend:**
- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

### Map display & interaction
- `●` Vector / topographic map rendering: MapLibre-GL vector base map (`osm-liberty` style) with aviation data drawn as GeoJSON overlay; ICAO-chart-like styling. Raster base maps also supported (`GeoMapProvider.currentRasterMap`).
- `◐` Terrain shading / elevation: Terrain MBTILES are downloaded and used for AGL altitude, airspace MSL estimation and the side-view profile (`GeoMapProvider.terrainElevationAMSL`), but relief/hillshade on the main map depends on the installed base-map style rather than being a dedicated shading layer.
- `◐` Map orientation (track / north / target up): North-Up, Track-Up and User-Defined-Bearing-Up (`MFM.qml` `MapBearingPolicies { NUp, TTUp, UserDefinedBearingUp }`). No "target up".
- `○` Auto-zoom & circling zoom (mode-dependent): Only a "follow GPS" auto-pan mode; no automatic or circling-dependent zoom.
- `●` Manual pan & zoom: Pinch/scroll-wheel zoom, pinch-rotate, and on-screen zoom buttons.
- `○` Snail trail / flight trail: No historical breadcrumb behind the aircraft. (A *forward* 5-minute flight vector is drawn instead — see FlightVector.)
- `○` Glide range ("reachability") line: Not present (soaring concept).
- `○` Thermal markers on map: Not present.
- `○` Markers / pilot events on map: No user-placed markers or pilot-event pins.
- `●` Waypoint & landable symbology / labels: ICAO-style airfield/navaid/reporting-point icons with zoom-dependent labels (`FlightMap.qml` waypoint layers, `src/icons/waypoints`).
- `●` "What's here" query: Tapping the map returns info on airspaces, airfields and navaids at that point (store description confirms; `WaypointDescription.qml`).

### Waypoints & navigation
- `●` Waypoint database: Aviation-map waypoints plus a user Waypoint library (`WaypointLibrary`, CUP/GPX/GeoJSON import).
- `●` Landable vs non-landable distinction: Airfields/aerodromes vs navaids/reporting points; "closest airfields for landing" workflow.
- `●` Nearest (waypoint / landable / airfield): `Nearby.qml` lists nearest aerodromes, waypoints and navaids with a name filter.
- `●` Go-to / direct navigation: `FlightRoute.directTo(waypoint, position)` from the waypoint description dialog ("Direct" button).
- `●` Waypoint details (info, image, runway, freq): `Waypoint.tabularDescription` exposes airport frequency, runway, elevation, ICAO code and magnetic variation. No per-waypoint photo (approach imagery is handled separately via VAC).
- `◐` Alternates / safety-landing selection: "Nearby / closest airfields for landing" covers the safety-landing use case, but there is no formal alternate/diversion object attached to a route.

### Cross-country tasks
- `○` Racing tasks: No task/scoring engine.
- `○` Assign Area Tasks (AAT): Not present.
- `○` Cross-country task types (FAI triangle, out-and-return, DMSt, etc.): Not present.
- `○` Observation zones & start/finish/turnpoint rules: No OZ concept.
- `◐` Task manager (build / edit): No *task* manager, but a **Flight-Route editor** exists (`FlightRouteEditor.qml`) — add/reorder/reverse waypoints, add by double-tapping the map. It is a nav route (sequence of legs), not a scored task.
- `○` Task calculator (in-flight required speeds etc.): No required-speed/AAT calculator. (Route ETE/ETA/fuel exists — see Glide computer / Data fields.)
- `◐` In-flight task edit: The flight route can be edited in flight (double-tap map to insert a waypoint), but there is no task to edit.
- `○` FAI badge / record support: Not present.
- `◐` Task import / export / declaration to logger: Route import/export in GA formats (GPX, FPL, PLN) and TripKit import, but no OZ-aware task and no declaration to an external logger.

### Glide computer
- `◐` Flight modes + auto display switching (cruise/circling/final): Only a coarse `FlightStatus { Ground, Flight }` (speed-hysteresis based). No cruise/circling/final-glide display modes.
- `○` MacCready setting (manual): Not present.
- `○` Auto MacCready (modes): Not present.
- `○` Glide polar (library + custom): Not present.
- `○` Ballast / bugs / weight setup: Not present.
- `○` Speed to fly / speed command / risk factor: Not present.
- `○` Safety heights / safety MacCready: Not present.
- `○` Final glide calculator (wind, altitude required): Not present.
- `◐` Task speed estimation: No task-speed model, but per-leg and remaining-route **ETE/ETA** are computed from cruise speed + manual wind (`Leg.ETE(wind, aircraft)`, `RemainingRouteInfo`).
- `○` Optimal cruise track: No optimizer; the route is flown as entered.

### Atmosphere & instruments
- `○` Variometer display: Not present.
- `○` Average climb: Not present.
- `○` Audio variometer: Not present.
- `◐` Air-data inputs (external sensors): Device barometric pressure/temperature sensors → pressure altitude / cabin altitude (`Sensors.h`, `QPressureSensor`); FLARM/GDL90 receivers also supply pressure altitude. No airspeed/TE-vario/air-data computer.
- `○` Wind estimation (circling / zigzag / compass / external): No automatic wind estimation. Wind is entered manually.
- `●` Wind display & manual override: Manual wind (speed + direction-from) set in the route editor's "Wind" tab and used in all leg/route calculations (`Navigator.setWind`, `Wind.h`).
- `○` Thermal locator / assistant / centering aid: Not present.
- `○` Thermal profile: Not present.
- `○` Convection forecast (cloud base etc.): Not present.

### Weather
- `●` METAR / TAF: Full decode & display (`weather/METAR.cpp`, `TAF.cpp`, `Decoder.cpp`, `Weather.qml`, `MetarTafDialog.qml`); QNH and density-altitude derived from station data.
- `○` Forecast overlays (SkySight / TopMeteo / RASP): Not present.
- `◐` Wind aloft / weather-station data: Surface weather-station data via METAR (incl. surface wind & QNH); no winds-aloft product.
- `●` In-flight weather updates: `WeatherDataProvider` runs regional background refresh of METAR/TAF and ages out stale reports.

### Airspace
- `●` Airspace display (classes, filtering): OpenAir + aviation-map airspaces (`geomaps/OpenAir.cpp`, `Airspace.h`), altitude-limit filter (with auto-raise on approach), "hide gliding sectors" toggle, night-mode styling.
- `○` Proximity / incursion warnings + acknowledgement: No active airspace-incursion detector. A `Warning_Navigation` notification category exists in code (enum comment references "Prohibited Airspace 1 minutes ahead") and is user-selectable for voice, but no generator emits it — no code path produces an airspace-ahead warning.
- `●` Airspace query / details: Tap a airspace to see name, class (CAT) and upper/lower bounds (imperial + metric).
- `●` NOTAM handling: `NOTAMProvider`/`NOTAMList`, on-map NOTAM icons (obstacle/UAV/etc.), list dialog, classification and abbreviation expansion (`expandNotamAbbreviations`).

### Traffic & collision awareness (FLARM)
- `●` FLARM Traffic on map: First-class; typed traffic icons (glider, jet, copter, drone, balloon, paraglider, hang-glider) in `src/icons/`.
- `●` OGN Traffic on map: Internet-based OGN source (`TrafficDataSource_Ogn.cpp`).
- `●` FLARM and OGN Traffic deduplication: "Avoid showing traffic twice" (CHANGELOG 3.3.0).
- `○` FLARM radar view: No dedicated relative-position radar screen. Traffic is shown on the moving map; traffic *without* position is drawn as a proximity ring around ownship.
- `◐` FLARM traffic dead reckoning: Traffic is animated/extrapolated between updates on the moving map (CHANGELOG 3.3.1 "Animate traffic").
- `●` Collision / proximity warnings: FLARM alarm-level warnings with relative bearing / vertical / distance (`traffic/Warning.cpp`), surfaced visually and optionally spoken (voice notifications).
- `●` Traffic details dialogue: Traffic labels on map plus the Traffic-Receiver page listing objects with hDist/vDist (`TrafficReceiver.qml`).
- `●` Traffic ID / registration lookup: FLARMnet DB (FLARM-ID → registration, `FlarmnetDB.cpp`) and a transponder DB (`TransponderDB.cpp`).
- `○` Team flying / buddy codes: Not present.

### Avionics & airframe
- `◐` Battery / voltage monitoring: No airframe voltage. Traffic-receiver status/self-test results are reported (`trafficReceiverSelfTestError`, `statusString`), and some receivers report their own status.
- `●` GPS status / connection / altitude source: `Positioning.qml` shows satellite/position status; source selectable between internal GNSS and traffic receiver; geoid correction (`positioning/Geoid.cpp`).
- `○` Engine / powered flight (ENL, MoP, engine hours): Not present.
- `○` Radio frequency management / control: Frequencies are *displayed* for airfields, but there is no radio control.
- `○` Transponder (XPDR) control / squawk: The aircraft profile stores a squawk code (`Aircraft.transponderCode`) for reference/flight-plan, but there is no transponder hardware control.
- `○` Switch inputs (gear / flap warnings etc.): Not present.
- `◐` Multiple external devices / slave mode: Multiple traffic/position data sources can be configured simultaneously (BT-classic, BLE, serial/USB, TCP, UDP) with automatic selection of the source currently sending heartbeats; no master/slave display arrangement.

### Data fields (InfoBox system)
- `○` Configurable data-field grid: No InfoBox system. Navigation values live in fixed UI chrome — the Remaining-Route bar, map scale, side-view and dialogs.
- `○` Multiple data-field pages / layouts: Not present.
- `○` Per-flight-mode auto layout: Not present.
- `◐` Altitude / altimetry values (QNH, pressure alt, FL, AGL): MSL/AGL altitude (toggle `showAltitudeAGL`), a dedicated Pressure-Altitude tool (`PressureAltitude.qml`), and QNH from weather — but shown in fixed places, not configurable fields.
- `◐` Speed values (GS / TAS / IAS / optimal): Ground speed available; no TAS/IAS/optimal.
- `◐` Direction values (track / bearing / heading): Track / true course / bearing shown; true heading + wind-correction angle computed per leg (`Leg`), but not as configurable fields.
- `●` Time values (UTC / local / ETA / ETE / sunset / flight time): ETA/ETE for next & final waypoint (`RemainingRouteInfo`), sunrise/sunset info (`WeatherDataProvider.sunInfo`), and a clock (`navigation/Clock.cpp`).
- `◐` Touch / gesture interaction with data fields: Map elements are tappable; the route bar is informational, not an interactive field grid.

### Analysis & review
- `○` Barograph / altitude trace: Not present.
- `○` Climb history / thermal analysis: Not present.
- `○` Wind analysis: Not present (wind is manual).
- `○` Glide polar analysis: Not present.
- `○` Task analysis: Not present.
- `○` OLC / contest analysis: Not present.
- `●` Airspace cross-section: **Side View** (`ui/SideviewQuickItem.cpp`, `Sideview.qml`) draws a live lateral profile of terrain and airspaces ahead, colour-coded by class, with a 5-minute range bar.

### Contest / WeGlide optimization (live)
- `○` Live WeGlide optimization in-flight: Not present.
- `○` Flight trace maintenance: Not present.
- `○` Live scoring / achieved distance: Not present.

### Live tracking
- `○` Live tracking upload (OGN / SkyLines / cloud): Not present — the app is explicitly non-commercial and collects no user data (OGN is receive-only).
- `◐` Retrieve / crew comms / position sharing: Own position/location can be shared to external apps (mapy.com, `geo:` URLs) and files shared via the platform share sheet, but there is no retrieve/crew coordination system.

### Instrument & device connectivity (I/O)
- `●` Bluetooth: Bluetooth Classic / SPP (`TrafficDataSource_BluetoothClassic.cpp`, `ConnectionScanner_Bluetooth.cpp`).
- `●` BLE: Bluetooth Low Energy source (`TrafficDataSource_BluetoothLowEnergy.cpp`).
- `●` Serial: Serial-port source, including Android USB-serial (`TrafficDataSource_SerialPort.cpp`, `ConnectionScanner_SerialPort.cpp`).
- `●` USB: Android USB-serial via a Kotlin/JNI helper (`UsbSerialHelper`).
- `●` TCP/IP / network: TCP and UDP sources (`TrafficDataSource_Tcp.cpp`, `_Udp.cpp`); an internal tile server also serves map tiles over localhost.
- `●` NMEA input parsing: FLARM NMEA parsing (`TrafficDataSource_Abstract_FLARM.cpp`) plus GDL90 and X-GPS decoders.
- `○` NMEA / data output (drive other devices): Consumer only; no NMEA/data output to slave other instruments.
- `●` Positioning source selection (internal / external): Internal GNSS or traffic-receiver position (`GlobalSettings.positioningByTrafficDataReceiver`).
- `●` Vendor protocols (LXNav / FLARM / …): FLARM (NMEA), GDL90 (Garmin/uAvionix), X-GPS (Dual/SkyEcho). LXNav-class devices work through their FLARM-format NMEA output; no additional device-proprietary protocol beyond these.

### Data & file management
- `●` Waypoint files (CUP / …): CUP (`fileFormats/CUP.cpp`), plus GPX and GeoJSON; imported into the Waypoint library.
- `●` Airspace files (OpenAir / CUB / …): OpenAir (`geomaps/OpenAir.cpp`). No CUB.
- `●` Terrain / topology / map data: Vector + raster base maps and terrain, all MBTILES; aviation maps as a multi-file set (`DataManager`).
- `●` In-app data download & updates: `DataManagerPage.qml` / `DataManager`; near-weekly aviation-map updates with in-app update notifications.
- `◐` Aircraft / plane profiles (polar, ballast, reg, comp ID): Aircraft library with per-aircraft name, cruise/descent/minimum speed, fuel consumption, units and squawk (`navigation/Aircraft.h`). No polar, ballast, registration or competition ID (powered-GA model).
- `○` Handicap / polar lists: Not present.

### Logging & recording
- `○` IGC flight log recording: Not present.
- `○` Approved / signed logger (badge / record): Not present.
- `○` Flight replay: No user flight replay. (There is a demo/simulation mode that plays back recorded NMEA/traffic files for testing/screenshots — see 3.99.)
- `○` Pilot events / markers logging: Not present.

### Configuration & UI
- `◐` Configuration profiles (per user / per aircraft / global): Per-aircraft profiles (Aircraft library) and saved routes (Flight-Route library) on top of global settings; no per-user profiles.
- `○` Screen layout / data-field geometry: Fixed layout; not user-configurable.
- `◐` Day / night / high-contrast modes: Dedicated Night Mode tuned for dark-adapted VFR-night eyes (`nightMode`). No separate high-contrast mode.
- `●` Units: Per-aircraft horizontal (nm/km/…), vertical (ft/m) and fuel (l/gal) units, plus imperial+metric airspace bounds and a font-size setting.
- `◐` Input / gestures / hardware buttons: Rich touch gestures (pinch zoom, pinch-rotate, double-tap to add waypoint); no hardware-button mapping.
- `●` Language / localization: Extensive Qt Linguist translations (`buildscript-translations.sh`, `pull-translations.sh`).

### Additional features (outside the shared taxonomy)

- `●` Visual Approach Charts (VAC): Georeferenced approach-chart library — import individual GeoTIFFs or TripKit bundles, rename, list by distance, and display overlaid on the moving map (`geomaps/VAC*.cpp`, `VAC.qml`).
- `●` Side View: Live lateral terrain + airspace cross-section with a 5-minute bar (also listed under 3.11).
- `●` Pressure Altitude tool: Standalone cabin-/pressure-altitude readout from the device barometric sensor (`PressureAltitude.qml`).
- `●` Demo / simulation mode: `DemoRunner`, `TrafficDataSource_Simulate` and `TrafficDataSource_File` replay recorded position/traffic data for testing and screenshots (not a user flight replay).
- `●` GA route file interchange: Import/export/share flight routes as GPX, FPL (Garmin) and PLN (MSFS / Little Navmap); TripKit import.
- `●` Density-altitude computation: `weather/DensityAltitude.cpp`, surfaced alongside weather.
- `●` Location sharing to external apps: Share own position via `geo:` URLs and partner apps (e.g. mapy.com).
- `●` Privacy-first design: No account, no telemetry, no user-data collection (detailed `PRIVACY.md`); works fully offline after data download.
- `●` Broad platform reach: Single codebase shipping to Android, iOS, Linux (Flathub), macOS and Windows.

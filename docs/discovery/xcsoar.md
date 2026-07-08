# Discovery: XCSoar

## System overview

- **Version(s) examined:** 7.45 (git `main`, HEAD `a177e7b`, 2026-07-05)
- **Date(s) examined:** 2026-07-06
- **Platform(s):** Android 6.0+, iOS, Windows (Vista+), Unix/Linux, macOS, Kobo eReader; embedded/single-board (Raspberry Pi, OpenVario). Distributed via F-Droid, Google Play, App Store, and direct download.
- **License:** GPLv2 (GNU GPL, Version 2). Third-party components noted in `THIRD_PARTY_NOTICES.txt`.
- **Offline behavior:** Fully offline by design ("Works offline; your flight data stays on your device"). All core navigation, glide-computer, task, airspace, FLARM and logging functions run with no connectivity. Internet is only used for _optional_ downloads: map/site-file/RASP updates, NOTAM areas, METAR/TAF, and live tracking upload.
- **Configuration model:** Text-based profiles (`.prf`, `<label>=<value>`, SI units), multiple profiles selectable at startup and shareable between pilots; separate plane files (`.xcp`). Per-flight-mode InfoBox sets, up to 8 screen pages, and a deep hierarchy of configuration dialogs (Site Files, Map Display, Glide Computer, Gauges, Task Defaults, Look, Setup, Plane, Devices). Overall complexity is high, with heavy reliance on automatic behavior to reduce in-flight interaction.

## Feature inventory

**Status legend:**

- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

### Map display & interaction

- `●` Vector / topographic map rendering: ESRI Shapefiles generated from OpenStreetMap (roads, rivers, lakes, cities, towns), packaged in the `.xcm` map database; cities/towns labelled in italics.
- `●` Terrain shading / elevation: raster DEM (GeoJPEG2000, ~2° geoid grid), elevation colouring, slope shading selectable by sun ephemeris / wind direction / fixed; brightness and amount configurable; sub-sea-level/invalid terrain drawn blue.
- `●` Map orientation (track / north / target up): all three, with an independent orientation selectable for circling mode; glider position on screen adjustable (e.g. 20% from bottom in cruise).
- `●` Auto-zoom & circling zoom (mode-dependent): separate circling vs cruise/final zoom levels; Auto Zoom scales to keep next waypoint visible and centres on detected thermal while circling.
- `●` Manual pan & zoom: gestures (up/down), buttons, mouse wheel, Android +/- rocker; dedicated pan mode ("P" gesture) with crosshair focus.
- `●` Snail trail / flight trail: Off / Short (~10 min) / Long (~1 h) / Full; coloured/thicknessed by altitude or (netto) vario; optional wind-drift compensation while circling; auto-shortened in circling to reduce clutter.
- `●` Glide range ("reachability") line: reach "footprint" as black/white dashed ring; straight-line or turning-reach (max 3 turns, ≤90°) with terrain avoidance and holes for unreachable peaks; red cross at terrain-clearance violation; optional blur of unreachable area.
- `●` Thermal markers on map: last 20 thermals stored per flight, drift-compensated to project the source position to current altitude/wind.
- `●` Markers / pilot events on map: manual or automatic markers drawn as flags (e.g. auto-drop on entering circling), appended to `xcsoar-marks.txt`; Pilot Event (PEV) system.
- `●` Waypoint & landable symbology / labels: 3 landable icon sets (Purple Circle / B&W / Traffic-lights) with landable/marginal/reachable states; "Detailed landables" encodes runway direction; several label abbreviation schemes and declutter modes; arrival-height and/or glide-ratio labels.
- `●` "What's here" query: touching any map point opens the "Map elements at this location" dialog (airports, airspaces, weather, etc.), even during pan.

### Waypoints & navigation

- `●` Waypoint database: multiple waypoint files usable simultaneously, or loaded from the `.xcm` map database if none configured; "Watched waypoints" file forces arrival-height labels even when unreachable.
- `●` Landable vs non-landable distinction: core distinction driving symbology; non-landables get type-specific icons (mountain top, obstacle, pass, power plant, tower, tunnel, weather station, bridge).
- `●` Nearest (waypoint / landable / airfield): nearest-landables logic drives abort mode, alternates and map arrival heights; nearest waypoint shown in Flight status.
- `●` Go-to / direct navigation: Goto tasks (including automatic goto to takeoff/nearest airfield).
- `●` Waypoint details (info, image, runway, freq): details dialog with airfield information from the Airfields/Waypoint-details file, optional image, runway and frequency data (e.g. from `.cup`).
- `●` Alternates / safety-landing selection: dedicated Alternates dialogue and Abort mode presenting nearest landing options.

### Cross-country tasks

- `●` Racing tasks: full "assigned task" with arm/disarm start, start/finish open/close times, max start speed/height (AGL/MSL), min finish height, PEV start.
- `●` Assign Area Tasks (AAT): AAT / MAT / TAT with min task time, area cylinders/sectors, in-flight target moving and target projection, task-calculator integration.
- `●` Cross-country task types (FAI triangle, out-and-return, DMSt, etc.): built via the task editor (any ordered shape); dedicated contest optimizer covers OLC (Sprint/FAI/Classic/League/Plus), DMSt (Free/OR/Quad/Triangle), WeGlide (Distance/FAI/Free/OR), XContest (Free/Triangle), NetCoupe, Charron, SISAT.
- `●` Observation zones & start/finish/turnpoint rules: Start cylinder/line; FAI sector, DAeC keyhole (0.5/10), BGA fixed-course and enhanced-option sectors, turnpoint cylinder, symmetric quadrant, area cylinder/sector; FAI finish quadrant / finish line / finish cylinder; BGA and DAeC sector support.
- `●` Task manager (build / edit): full task manager (Turn Points, Manage, Rules, Task Calculator tabs) with save/name/revert.
- `●` Task calculator (in-flight required speeds etc.): task-calc dialog for MC, AAT range, required speeds.
- `●` In-flight task edit: supported; modifying a declared task raises a confirmation warning to avoid invalidating the declaration.
- `◐` FAI badge / record support: "FAI badges/records" is a selectable task type with FAI start/finish rules, but the internal software logger is _not_ IGC-certified — official badges/records require an approved external logger.
- `●` Task import / export / declaration to logger: task files saved/loaded; declaration sent to many supported external loggers; auto-declares current task on auto-start.

### Glide computer

- `●` Flight modes + auto display switching (cruise/circling/final): four modes (Cruise / Circling / Final glide / Abort); circling auto-detected (~¾ turn on, ~30 s straight off) or via external switch; mode drives InfoBox set, zoom, vario gauge, thermal assistant, trail drift.
- `●` MacCready setting (manual): via InfoBox arrows or synced from a connected intelligent vario.
- `●` Auto MacCready (modes): Final glide (min time / OLC-sprint distance), Trending average climb, or Both (default); propagated to the vario.
- `●` Glide polar (library + custom): large built-in polar library (`PolarStore`) plus fully custom polars.
- `●` Ballast / bugs / weight setup: flight-setup dialogue for ballast, bugs, QNH, max temperature; ballast dump manager.
- `●` Speed to fly / speed command / risk factor: STF with chevron speed command; "speed to fly with risk" factor.
- `●` Safety heights / safety MacCready: arrival height, terrain safety/clearance heights, break-off height, safety MC (used for reach and abort).
- `●` Final glide calculator (wind, altitude required): final-glide bar with required-altitude-difference display, optional dual altitude-required bars.
- `●` Task speed estimation: task-speed estimation and analysis page.
- `●` Optimal cruise track: computed optimal cruise track (dedicated section).

### Atmosphere & instruments

- `●` Variometer display: needle-dial gauge, gross reading + instantaneous text + speed-command chevrons; GPS-estimated when no intelligent vario connected.
- `●` Average climb: 30 s averager — gross climb while circling, netto (airmass) while cruising; optional caret needle.
- `◐` Audio variometer: present but Android-only; needs a barometric sensor (external or device-internal).
- `●` Air-data inputs (external sensors): consumes TE/gross vario, netto vario, aircraft acceleration/load factor, barometric altitude, IAS, air density from intelligent varios.
- `●` Wind estimation (circling / zigzag / compass / external): circling algorithm (all installs; improved with airspeed/IMU), zigzag (needs TAS), compass algorithm (in development); altitude-banded wind statistics.
- `●` Wind display & manual override: on-map wind vector + numeric; manual override via wind dialogue or InfoBox cursor keys; auto-wind mode selectable (Manual/Circling/Zigzag/Both).
- `●` Thermal locator / assistant / centering aid: thermal locator (green spiral marker, drift-compensated) plus graphical thermal assistant polar diagram (corner + full-screen).
- `●` Thermal profile: thermal-band meter (height band vs average climb, MC-scaled) drawn left of the final-glide bar.
- `●` Convection forecast (cloud base etc.): estimates convection ceiling and cloud base from OAT/humidity probe + forecast max ground temperature; temperature-trace analysis page.

### Weather

- `●` METAR / TAF: downloadable per ICAO station (needs internet), shown as weather-station flags and in map-element/details dialogs.
- `◐` Forecast overlays (SkySight / TopMeteo / RASP): RASP (built-in auto-download, colour-coded terrain overlays, many fields) and EDL Soaring wave overlays (MBTiles) supported; pc_met (Deutscher Wetterdienst) integration. SkySight/TopMeteo are _not_ natively integrated.
- `●` Wind aloft / weather-station data: weather-station data via map flags; RASP wind fields; "Wind at altitude" analysis page.
- `●` In-flight weather updates: in-flight RASP/EDL forecast-time controls (Bottom area = "Weather controls"); manual METAR/TAF update.

### Airspace

- `●` Airspace display (classes, filtering): shaded areas with class-specific colours/patterns (opaque/hollow/hatched/stippled); filter by all / below altitude / height separation / below glider; ICAO-consistent default colouring; per-class enable/disable.
- `●` Proximity / incursion warnings + acknowledgement: predicted-incursion (uses long-term average track, works while circling/drifting), entering, and leaving events; graded warnings (None/Near/Inside) with an acknowledgement dialogue; uses baro altitude (QNH) preferentially; warns even when off-screen.
- `●` Airspace query / details: airspace filter dialogue, selection dialogue, and details view.
- `●` NOTAM handling: downloads NOTAM-affected areas as airspace with an incremental/cached model, configurable radius/refresh, default filters (IFR-only, ineffective, >100 km, admin Q-codes), and NOTAM-specific details (number, ICAO, Q-code, validity, altitudes, text). Situational-awareness only — does not replace a NOTAM briefing.

### Traffic & collision awareness (FLARM)

- `●` FLARM Traffic on map: red arrowheads oriented to display orientation, coloured by threat level; optional aircraft-registration/pilot-name labels (FlarmNet/local file, respecting the FLARM privacy flag).
- `◐` OGN Traffic on map: available via GliderLink (an Android companion app that broadcasts OGN traffic); drawn as a separate GliderLink traffic layer, Android-oriented.
- `○` FLARM and OGN Traffic deduplication: FLARM and GliderLink/OGN targets are maintained and drawn as separate lists; no deduplication logic found.
- `●` FLARM radar view: track-up radar "rose" (linear to 2000 m, 1000/2000 m rings, disappears >4000 m); enlargeable to a full-screen FLARM Traffic dialogue with per-target detail and gestures.
- `○` FLARM traffic dead reckoning: no position extrapolation between updates found; last-known position is drawn.
- `●` Collision / proximity warnings: threat-level colouring (yellow L1, red L2/3) and warning circles; the manual explicitly advises relying on a FLARM _audio_ device for actual collision avoidance.
- `●` Traffic details dialogue: per-target details (ID, vario, distance, relative height) in radar corners and a separate details dialog.
- `●` Traffic ID / registration lookup: FlarmNet database + XCSoar's own FLARM ID file lookup by competition ID.
- `●` Team flying / buddy codes: 5-digit team code relative to a reference waypoint; FlarmNet "lock" of a team-mate; unlimited "friends" by FLARM ID with custom colours.

### Avionics & airframe

- `●` Battery / voltage monitoring: Battery InfoBox (charge %, AC/charging state) on embedded platforms; low-battery detection and screen blanking to conserve power.
- `●` GPS status / connection / altitude source: status icons (acquiring / disconnected), multi-source redundancy (Device A primary, B secondary, auto-failover/auto-reconnect), WGS84 ellipsoid→geoid correction for GPS altitude.
- `◐` Engine / powered flight (ENL, MoP, engine hours): ENL (engine noise level) is received and forwarded (e.g. to SkyLines tracking) and can auto-start-only the logger for para-gliders; no first-class engine-hours / means-of-propulsion tracking or dedicated powered-flight UI found.
- `◐` Radio frequency management / control: Active/Standby-frequency InfoBoxes and an edit panel; actual radio control depends on a supported device driver (ATR833, KRT2, XCOM760, Air Control Display, LX).
- `◐` Transponder (XPDR) control / squawk: Transponder-code InfoBox and control via supported drivers (Air Control Display, LX); not universal.
- `◐` Switch inputs (gear / flap warnings etc.): airframe switch monitoring (airbrake, flap position, landing gear) plus derived alerts, largely via Triadis Vega over NMEA; a dedicated Vega switch dialogue exists.
- `●` Multiple external devices / slave mode: multiple simultaneous devices with value-preference merging; "NMEA Out" master→slave mode to drive a second unit.

### Data fields (InfoBox system)

- `●` Configurable data-field grid: many selectable InfoBox geometries sized to the screen.
- `●` Multiple data-field pages / layouts: up to 8 screen pages, each a map + InfoBox-set + optional bottom area / overlay.
- `●` Per-flight-mode auto layout: predefined Circling/Cruise/Final-glide InfoBox sets plus up to 5 custom; "auto" pages switch set by flight mode.
- `●` Altitude / altimetry values (QNH, pressure alt, FL, AGL): full Altitude InfoBox category.
- `●` Speed values (GS / TAS / IAS / optimal): ground speed, TAS, IAS (from vario), optimal/command speeds.
- `●` Direction values (track / bearing / heading): track, bearing, bearing difference, heading (with wind).
- `●` Time values (UTC / local / ETA / ETE / sunset / flight time): comprehensive time InfoBoxes incl. sunset via sun ephemeris.
- `●` Touch / gesture interaction with data fields: tap an InfoBox to select/open its panel; gestures and simulation-mode arrow interaction.

### Analysis & review

- `●` Barograph / altitude trace: altitude history with estimated working band (base/ceiling) and ceiling trend; settings shortcut.
- `●` Climb history / thermal analysis: per-climb bar chart with average/trend and MC line; separate thermal-band page.
- `●` Wind analysis: "Wind at altitude" page (speed vs altitude, vectors) with set-wind shortcut.
- `●` Glide polar analysis: dedicated Polar page (plus MacCready page).
- `●` Task analysis: Task page (AAT areas shaded, remaining-target path) and Task-speed page; task-calc shortcut.
- `●` OLC / contest analysis: Contest page with optimal path (thick red) and estimated distance/score for the selected rule set.
- `●` Airspace cross-section: Airspace cross-section page in the analysis dialog and an on-map cross-section bottom bar (terrain + airspace).
- _(Also present: Vario histogram, Temperature-trace pages.)_

### Contest / WeGlide optimization (live)

- `●` Live WeGlide optimization in-flight: continuous background optimization; contest rule set (OLC/WeGlide/DMSt/XContest/etc.) selectable via task rules.
- `●` Flight trace maintenance: aircraft track maintained (thin green) with optimal path (thick red) computed live.
- `●` Live scoring / achieved distance: instantaneous contest distance/score InfoBox; frozen after landing.

### Live tracking

- `●` Live tracking upload (OGN / SkyLines / cloud): SkyLines, LiveTrack24, and SkyLines "cloud" upload built in (need internet/cellular).
- `◐` Retrieve / crew comms / position sharing: SkyLines tracking shows other pilots' positions ("traffic") and shared thermals; no dedicated retrieve/crew-messaging feature found.

### Instrument & device connectivity (I/O)

- `●` Bluetooth: Android Bluetooth Classic (client + server port).
- `●` BLE: Android BLE serial port (`OpenAndroidBleSerialPort`).
- `●` Serial: native serial/TTY; Android IOIO UART.
- `●` USB: Android USB-serial port.
- `●` TCP/IP / network: TCP client and server, UDP.
- `●` NMEA input parsing: extensive NMEA sentence parsing across ~83 device drivers.
- `●` NMEA / data output (drive other devices): "NMEA Out" slave mode re-emits received/outgoing data to a second unit.
- `●` Positioning source selection (internal / external): internal GPS vs external device(s) with prioritized multi-source selection/failover.
- `●` Vendor protocols (LXNav / FLARM / …): ~83 drivers incl. LX, FLARM/PowerFLARM, CAI302, Vega, Flymaster, XCVario, Borgelt, Stratux, ATR833, KRT2, and more.

### Data & file management

- `●` Waypoint files (CUP / …): SeeYou `.cup`, WinPilot/Cambridge `.dat`, Zander `.wpz`, OziExplorer `.wpt`, GPSDump/FS/GEO/UTM `.wpt`.
- `●` Airspace files (OpenAir / CUB / …): OpenAir subset `.txt` (incl. `AF`/`AR` frequency extension) and Tim Newport-Pearce `.sua`; classes A–G + Prohibited/Danger/Restricted/CTR/Wave/TMZ/etc.
- `●` Terrain / topology / map data: single `.xcm` map database (GeoJPEG2000 terrain + ESRI-shapefile topography), custom builds via mapgen.xcsoar.org.
- `●` In-app data download & updates: file-download selectors driven by the XCSoar data-content repository manifest plus user-defined repository URLs; RASP regions auto-downloadable.
- `●` Aircraft / plane profiles (polar, ballast, reg, comp ID): plane files (`.xcp`) with polar, ballast, registration, competition ID, handicap; "Configure Plane" dialog.
- `●` Handicap / polar lists: built-in polar store; per-plane handicap factor applied to OLC/contest scoring.

### Logging & recording

- `●` IGC flight log recording: internal software logger (IGC-format), auto start/stop on takeoff/landing, ~60 s pre-takeoff buffer, auto free-space management (deletes oldest IGC files below 500 kB).
- `◐` Approved / signed logger (badge / record): internal logger is IGC-format but **not certified**; XCSoar supports declaration to, and IGC download from, many _external_ approved loggers for official badges/records.
- `●` Flight replay: IGC replay (any XCSoar/other IGC log), variable time scale (0×–N×), auto-stopped if real GPS detects motion in FLY mode.
- `●` Pilot events / markers logging: Pilot Event (PEV) with logger announcement to connected devices; manual/auto markers.

### Configuration & UI

- `●` Configuration profiles (per user / per aircraft / global): multiple `.prf` profiles selectable at startup, partial-overwrite semantics, shareable; separate `.xcp` plane profiles.
- `●` Screen layout / data-field geometry: InfoBox geometry, borders (Box/Tab), FLARM radar placement, dialog/message sizing, cursor size/colour.
- `◐` Day / night / high-contrast modes: inverse InfoBoxes (white-on-black), colour InfoBoxes, adjustable terrain brightness — but no dedicated day/night theme toggle or auto-brightness scheduler found.
- `●` Units: presets (American / Australian / British / European) plus per-item custom (speed, distance, lift, altitude, temperature, task speed, pressure, lat/lon incl. UTM).
- `●` Input / gestures / hardware buttons: customizable Input Events file (menus/buttons/external events), swipe gestures, hardware buttons, optional Android haptic feedback, selectable text-input style.
- `●` Language / localization: many bundled translations (Automatic / English / explicit language selection); status-message and font customization.

### Additional features (outside the shared taxonomy)

<!-- Anything this system does that has no row above. -->

- `●` Route planning (3D): plans paths around terrain and airspace (terrain-only or terrain+airspace) in ordered/abort/goto modes, minimum-time optimal, with configurable climb ceiling.
- `●` Embedded Lua scripting: bundled Lua engine (`src/lua/`) for automation/extension (airspace, background, timers, input events, etc.).
- `●` Attitude indicator / artificial horizon: Horizon InfoBox/widget computed from flight path + acceleration/vario.
- `●` ADS-B traffic: received via FLARM/PowerFLARM or Stratux (rendered through the FLARM traffic system).
- `●` Simulation (SIM) mode: on-device simulator for training (arrow-key control), independent of GPS.
- `●` Checklist file: user checklist (`xcsoar-checklist.txt`) accessible in-app.
- `●` OpenVario / Kobo system integration: dedicated `src/OV` and `kobo/` integration (system menus, shell integration) for these embedded devices.
- `●` Sun ephemeris & sunset warnings: sunrise/sunset computation with an "arrival past sunset" status warning.
- `●` Vega / CAI302 device management: special "Manage" dialogs (e.g. clear CAI302 flight memory, Vega airframe/setup).
- `●` Status dialogues: multi-tab Flight / Times / System / Rules status views (System page updates live).

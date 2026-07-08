# Discovery: LXNAV LX90xx / LX80xx

## System overview

- **Version(s) examined:** Firmware 9.5, manual Rev #59
- **Date(s) examined:** January 2025 (manual publication)
- **Platform(s):** Dedicated avionics hardware. Main display unit runs Linux (explicitly _not_ Windows CE) on an ARM processor; paired with a separate vario sensor unit (V8 / V80 / V9, ARM Cortex-M4). Not a phone/tablet app. Family spans LX9000/9050/9070 (large-screen, portrait or landscape, optional multitouch) and LX8030/8040 (smaller, landscape only).
- **License:** Proprietary commercial firmware bundled with the hardware. Feature tiers (AHRS, HAWK, Club options) are unlocked via paid activation codes tied to the unit serial number.
- **Offline behavior:** Fully self-contained for all core soaring functions. Worldwide terrain/vector maps, airspace and airport databases are pre-loaded and the internal IGC recorder, glide computer, FLARM, tasks and statistics need no connectivity. Internet (via an optional Wi-Fi module + phone hotspot) is only needed for live/forecast weather, cloud storage/sync, NOTAMs, SeeYou Cloud maps and firmware/database downloads. Weather is cached (≈500 km live radius, whole-sector forecast) so it remains usable after signal loss.
- **Configuration model:** Rich. Pilot/location **profiles** (`.lxprofile`) hold navpage layout + device settings; can be pin-locked, admin-locked (club mode disables individual menus per user), synced via LXNAV Connect. **Polar & Glider** data (up to 3 gliders, incl. weight & balance / CG) is stored separately on the device, not in the profile. Desktop customisation via **LX Styler**; layouts built from navboxes + graphical symbols. Two-seat front/rear units exchange selected data over an RS485 bus.

## Feature inventory

**Status legend:**

- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

### Map display & interaction

- `●` Vector / topographic map rendering: OSM-derived worldwide vector data — elevation contours, water bodies, roads/highways, railways, cities; per-element zoom/width/colour. Database not user-editable.
- `●` Terrain shading / elevation: full DEM with optional shadows, 3 detail levels + off, and many colour schemes (Mountain, Flatland, ICAO, Cliffs/Google-like, Atlas/Imhof, Grayscale, OSM, Himalaya, Zebra, plus a **Relative** altitude-reachability scheme). Optional HGL high-resolution elevation add-on.
- `●` Map orientation (track / north / target up): fixed N/E/S/W plus Track-up, Heading-up (compass or wind-derived) and Goal-up; configured separately for straight vs. circling.
- `●` Auto-zoom & circling zoom (mode-dependent): "Zoom to target" auto-scales to keep the goal visible (1–200 km); the thermal page uses its own fixed page-zoom and a separate circling orientation.
- `●` Manual pan & zoom: dedicated PAN mode (knobs or remote jogger), ZOOM knob; on touch models a long-press jumps straight to PAN.
- `●` Snail trail / flight trail: "Show path" with selectable length and colouring by Fixed / MacCready / Vario / Altitude / Ground-speed / HAWK-Netto; engine-on segments coloured separately.
- `●` Glide range ("reachability") line: "Show glider range area" fills the reachable area from current altitude using wind + safety MacCready; fill inside or outside.
- `◐` Thermal markers on map: a live circling **thermal assistant** (sized dots around the glider) plus thermal-column statistics/graph, but no persistent dropped thermal-hotspot markers left on the map.
- `●` Markers / pilot events on map: MARK creates a waypoint at present position; EVENT logs an IGC PEV (bumps logging to 1 Hz for a minute).
- `●` Waypoint & landable symbology / labels: per-type symbols and dual upper/lower labels (name/code/elevation/arrival alt/required alt/required Mc/required L/D/team code/frequency), reachability-coloured label backgrounds (green at current Mc, yellow at Mc 0), runway-direction symbols, red cross on too-short/too-narrow strips.
- `●` "What's here" query: PAN + INFO cycles waypoint / airspace / raw-position info at the cursor, with GOTO or airspace dismiss/frequency shortcuts.

### Waypoints & navigation

- `●` Waypoint database: unlimited waypoints; CUP, CUPX (with images), and Cambridge/WinPilot DAT (converted to CUP). Multiple files active at once.
- `●` Landable vs non-landable distinction: landable types recognised; min runway length/width filter marks unusable strips with a red cross; near-mode landables list.
- `●` Nearest (waypoint / landable / airfield): Near mode lists landables/airports, default-sorted by arrival altitude, sortable, GOTO direct.
- `●` Go-to / direct navigation: GOTO from any select method or from a FLARM/marked point.
- `●` Waypoint details (info, image, runway, freq): CUPX images, frequency, runway info and description shown on dedicated navpages.
- `●` Alternates / safety-landing selection: near-mode landables plus a favourites list that also includes take-off point and soaring-start ("where to finish" for OLC).

### Cross-country tasks

- `●` Racing tasks: full, with start-arm, gate intervals, max start altitude/speed, event-start procedure.
- `●` Assign Area Tasks (AAT): full — large zones with geometry, task time, AAT isolines (speed/Δtime/distance), optimal-track arrow, move-point-inside-area, min/max task distance navboxes.
- `●` Cross-country task types (FAI triangle, out-and-return, DMSt, etc.): tasks are built as arbitrary point sequences; a real-time **FAI-triangle assistant** and OLC/FAI optimisation cover free-flight XC. DMSt/handicap-distance scoring not called out by name.
- `●` Observation zones & start/finish/turnpoint rules: complete OZ model (Angle1/Angle2, Radius1/Radius2, Angle12 with Symmetric/Fixed/Next/Prev/Start orientation, line/cylinder/FAI sector), global defaults + per-point overrides, templates, multiple start points, auto-next.
- `●` Task manager (build / edit): list, detailed-list and map edit views; insert/move/delete/invert/clear; save to active waypoint file.
- `●` Task calculator (in-flight required speeds etc.): task speed achieved, required speed, remaining time/distance, task required Mc, AAT isolines.
- `●` In-flight task edit: allowed in the air (prepare/edit/load/save); declaration itself is fixed at take-off and in-air edits are not re-written to the IGC declaration.
- `●` FAI badge / record support: IGC-approved recorder; "Finish is below start" mode for badge/record finishes with safety-altitude handling.
- `●` Task import / export / declaration to logger: load from CUP/CUPX and from SoaringSpot; auto-declared in the IGC file at take-off; declaration push to a connected LX/Nano and optionally to external FLARM.

### Glide computer

- `●` Flight modes + auto display switching (cruise/circling/final): Auto-SC changeover by GPS-circling, g-load, IAS threshold, external switch or flap sensor; automatic thermal page; final-glide symbol always shown.
- `●` MacCready setting (manual): full, with wing-loading and resulting L/D shown.
- `◐` Auto MacCready (modes): the Mc dialogue _suggests_ a value from the last four thermals, and ETA methods can use vario-based climb; there is no fully automatic self-setting Mc.
- `●` Glide polar (library + custom): large predefined glider list plus custom a/b/c quadratic coefficients (LX-Polar helper), reference weight/wing area.
- `●` Ballast / bugs / weight setup: ballast as load or water litres (incl. tail water), measured water dump countdown, bugs as % L/D degradation.
- `●` Speed to fly / speed command / risk factor: STF value, reqSTF ("arrive at target at chosen altitude"), speed-to-fly audio and airspeed-tape marker. No XCSoar-style "risk factor" parameter.
- `●` Safety heights / safety MacCready: safety (arrival) altitude, plus independent **Safety Mc** and distance-dependent **Safety Mc-offset** applied to final glide only.
- `●` Final glide calculator (wind, altitude required): full, with green/yellow rectangles on course line marking Mc and Mc-0 achieve points, arrival/required-altitude navboxes.
- `●` Task speed estimation: task speed, required task speed, tETA/tETE.
- `●` Optimal cruise track: AAT optimal-track arrow and "Show optimal track" indicator.

### Atmosphere & instruments

- `●` Variometer display: needle-type vario indicator, vario tape and configurable vario meter symbol (vario/netto/relative/STF).
- `●` Average climb: configurable integrator (default 20 s), thermal average, red-diamond average on the indicator.
- `●` Audio variometer: extensive — multiple vario/SC audio modes, tunable frequencies at −100/0/+100 %, dead-band, equalizer, HAWK vs TE audio source.
- `●` Air-data inputs (external sensors): digital temp-compensated pressure sensors in vario units, 3-axis accel + gyros, OAT, AHRS, and up to 4 analog channels via LXDAQ/LXDAQ+.
- `●` Wind estimation (circling / zigzag / compass / external): four continuous methods — speed-difference (circling), position-drift, combination (with airspeed), and compass wind-triangle — plus HAWK; stored in 300 m / 1000 ft layers.
- `●` Wind display & manual override: wind profile display and dialogue, manual per-layer edit, and freehand draw on touch models.
- `●` Thermal locator / assistant / centering aid: graphical thermal assistant (bubble sizing by strength) and an audible thermal assistant; optional beep-before-max.
- `●` Thermal profile: thermal graph / statistics showing entry/exit, strongest/weakest points, column shape vs. altitude, Mc-based colouring.
- `◐` Convection forecast (cloud base etc.): not modelled onboard, but available via SkySight/TopMeteo layers (Cu cloud base, thermal height/depth, etc.) and a Potential-Temperature navbox as a trigger aid; requires a weather subscription.

### Weather

- `●` METAR / TAF: target-parsed METAR and TAF navboxes; meteogram for airports with a valid ICAO sign.
- `●` Forecast overlays (SkySight / TopMeteo / RASP): SkySight and TopMeteo forecast + satellite layers (RASP not supported); opacity, animation history-span, forecast-time offset.
- `●` Wind aloft / weather-station data: wind-aloft layers from SkySight/TopMeteo at multiple levels; live wind navbox.
- `●` In-flight weather updates: live satellite, forecast and rain-radar over the map via Wi-Fi + hotspot; rain radar needs no third-party login.

### Airspace

- `●` Airspace display (classes, filtering): per-type colour/width/transparency/zoom, "show only airspace below X", hide GPS-AeroData, show-inactive toggle; separate map vs. side-view styling.
- `●` Proximity / incursion warnings + acknowledgement: two-stage projected warnings (orange near, red imminent) with horizontal/vertical buffers; dismiss for minutes / today / quit, confirm-dismiss, reset-all.
- `●` Airspace query / details: vicinity list with vertical/horizontal distances and upper/lower limits, per-zone enable/disable (OFF today / OFF hh:mm / always), edit type/class/limits, name filter, side-view cross-section.
- `●` NOTAM handling: via GPS AeroData subscription — view/manage/filter NOTAMs (trigger / FIR-wide / aerodrome), sourced from Eurocontrol + national AIPs.

### Traffic & collision awareness (FLARM)

- `●` FLARM Traffic on map: internal or external FLARM; above/below/near colouring, labels, tracked paths, PCAS non-directional circles.
- `○` OGN Traffic on map: device _transmits_ to the OGN network (toggle "No tracking") but does not receive/display OGN traffic.
- `○` FLARM and OGN Traffic deduplication: n/a — no OGN traffic ingestion (duplicate removal exists only for waypoint-vs-database points, not traffic).
- `●` FLARM radar view: dedicated FLARM-radar symbol and navpage, plus split map+list view; ADS-B shown too.
- `◐` FLARM traffic dead reckoning: lost targets keep blinking for a configurable timeout (default 120 s) before removal; no explicit onboard dead-reckoning projection described.
- `●` Collision / proximity warnings: directed + undirected, low/medium/high alarm levels, obstacle warnings (with database), alert zones (drop/RPAS), ADS-B warnings, graphical + voice presentation, competition/reduce-warnings modes.
- `●` Traffic details dialogue: sorted FLARM list with ID/distance/bearing/vario/altitude, details and edit.
- `●` Traffic ID / registration lookup: pre-loaded FlarmNet database (updatable); competition-number display; per-target custom naming savable to SD/USB.
- `●` Team flying / buddy codes: encrypted Team code (SeeYou-compatible) for position sharing, favourite targets, and one "active" target driving team navboxes.

### Avionics & airframe

- `●` Battery / voltage monitoring: battery-type profiles (Lead/LiFe/LiPo/custom) with warnings and voltage offset; optional LXDAQ+ per-cell battery monitor (SOC, current, temp, remaining/charge time).
- `●` GPS status / connection / altitude source: GPS status page, satellite sky-view, single shared GPS receiver; selectable pressure-altitude source (IGC sensor vs. pitot-static vario sensor).
- `●` Engine / powered flight (ENL, MoP, engine hours): built-in ENL sensor + optional MOP box for jets, engine threshold/total-time, fuel-burn model, e-Glide energy reference voltage, "high ENL starts logger".
- `●` Radio frequency management / control: via 232 Bridge — KRT2, Trig TY91/92, ATR833, Becker 620X, ACD57; active/standby swap, dual watch, 8.33/25 kHz, history, auto-set target frequency.
- `●` Transponder (XPDR) control / squawk: via 232 Bridge — Becker BXP6402, Trig TT21/22; squawk/mode/ident/VFR-7000, ICAO ID.
- `●` Switch inputs (gear / flap warnings etc.): AGL-triggered gear warning (with speed/terrain/vario conditions), flap-position sensor + warnings, configurable digital inputs (SC, water valve, etc.).
- `●` Multiple external devices / slave mode: RS485 bus with splitters — rear/front repeater (…D units), secondary indicators (I8/I9/I80), remote sticks, compass, MOP, bridges, Wi-Fi, external FLARM.

### Data fields (InfoBox system)

- `●` Configurable data-field grid: navboxes with editable title/value/unit, global or per-box styling; ~150 defined types.
- `●` Multiple data-field pages / layouts: multiple navpages per mode (APT/WPT/TSK each have several), fully layout-editable.
- `◐` Per-flight-mode auto layout: a dedicated thermal page auto-activates on circling; cruise/final navpages don't auto-swap by mode otherwise.
- `●` Altitude / altimetry values (QNH, pressure alt, FL, AGL): Alt (MSL), AltIGC, AltGps, FL/FlIGC, RawIGC (above 1013), AGL/Height, QNH.
- `●` Speed values (GS / TAS / IAS / optimal): GS, TAS, IAS, GS-TAS, STF, reqSTF/trqSTF.
- `●` Direction values (track / bearing / heading): Trk, Brg, Hdg, To (steering course), Radial.
- `●` Time values (UTC / local / ETA / ETE / sunset / flight time): UTC, local Time/Date, ETA/ETE, tETA/tETE, Sunrise&Sunset, FltTime, TkOff.
- `◐` Touch / gesture interaction with data fields: multitouch is an option on LX90xx (config, task, pan, wind-draw); direct per-navbox touch editing in flight isn't emphasised, and LX80xx has no touch.

### Analysis & review

- `●` Barograph / altitude trace: logbook replay shows flown path + barogram, zoom/scrub through the flight.
- `●` Climb history / thermal analysis: thermal graph and in-flight thermal/leg statistics (average vario, overfly factor, leg efficiency).
- `●` Wind analysis: wind-profile symbol/dialogue by altitude layer.
- `◐` Glide polar analysis: polar dialogue shows best L/D and min-sink graphically for the entered polar; no post-flight measured-polar analysis.
- `●` Task analysis: detailed per-leg task statistics.
- `●` OLC / contest analysis: real-time OLC/FAI optimisation and OLC statistics page (legs/distance/speed).
- `●` Airspace cross-section: side-view symbol renders terrain + airspace toward the goal.

### Contest / WeGlide optimization (live)

- `●` Live WeGlide optimization in-flight: real-time distance optimisation to OLC or FAI rules onboard (3-point FAI free / 5-point OLC), incl. FAI-triangle assistant — not WeGlide-branded specifically.
- `●` Flight trace maintenance: optimisation and trace maintained continuously during flight; reset-on-engine-run option.
- `●` Live scoring / achieved distance: optimised distance, optimised speed/XC-speed, largest-triangle and OLC statistics updated live.

### Live tracking

- `◐` Live tracking upload (OGN / SkyLines / cloud): only indirectly — the FLARM transmitter makes the glider visible on OGN ground stations (unless "No tracking" is set). No SkyLines client; LXNAV Connect uploads the IGC _after landing_, not a live position stream.
- `◐` Retrieve / crew comms / position sharing: Team-code position sharing (voice-relayed, encrypted) and send-target-to-rear-seat; no dedicated crew retrieve/messaging channel.

### Instrument & device connectivity (I/O)

- `○` Bluetooth: not present.
- `○` BLE: not present.
- `●` Serial: RS232 (NMEA/PDA/Condor) and the LXNAV RS485 device bus.
- `●` USB: USB slot for memory-stick data transfer, updates and flight download.
- `◐` TCP/IP / network: Wi-Fi module provides IP networking for cloud services, sync and firmware/database download — not a general TCP NMEA link.
- `●` NMEA input parsing: parses FLARM (PFLAU/PFLAA) and vario NMEA; external-FLARM and PDA inputs.
- `●` NMEA / data output (drive other devices): NMEA output selectable per port with GPS / LXNAV / FLARM sentence groups and baud rate (drives PDAs, external FLARM displays, etc.).
- `●` Positioning source selection (internal / external): internal vs. external FLARM port selection, GPS source, and pressure-altitude source selection.
- `●` Vendor protocols (LXNav / FLARM / …): native LXNAV RS485 bus, FLARM, vario units, plus radio/XPDR vendor bridges (KRT2/Trig/Becker/ATR833/ACD57).

### Data & file management

- `●` Waypoint files (CUP / …): SeeYou CUP, CUPX (with images/passwords), Cambridge/WinPilot DAT.
- `●` Airspace files (OpenAir / CUB / …): user airspace as CUB (all types) plus LXNAV's free worldwide ASAPT airspace database; OpenAir not named explicitly.
- `●` Terrain / topology / map data: pre-loaded worldwide vector + DEM; optional HGL high-res elevation; raster CMR (SeeYou) and QMP (Ifos, serial-locked) scanned maps; SeeYou Cloud tile maps.
- `●` In-app data download & updates: ASAPT database updates from SD/USB, plus Wi-Fi/LXNAV Connect download & auto-sync of airspace, waypoints, maps, profiles and weather.
- `●` Aircraft / plane profiles (polar, ballast, reg, comp ID): up to 3 gliders stored on the device with polar, speeds, dump rates and full weight & balance / CG; sharable as `.lxg`.
- `◐` Handicap / polar lists: extensive predefined polar library; no DAeC/index handicap list surfaced in the manual.

### Logging & recording

- `●` IGC flight log recording: built-in recorder, selectable interval, logs wind/speed/vertical-speed by default, optional FLARM-data and flap logging.
- `●` Approved / signed logger (badge / record): high-level IGC approval with digital+mechanical security ("Calculating security" on landing); valid for records/badges.
- `●` Flight replay: logbook VIEW replays map + barogram with statistics/optimisation/task overlays.
- `●` Pilot events / markers logging: PEV via EVENT (1 Hz burst), MARK waypoints.

### Configuration & UI

- `●` Configuration profiles (per user / per aircraft / global): pilot/location profiles, pin-lockable and admin-lockable (club mode), cloud-syncable; polar/glider kept separately per device.
- `●` Screen layout / data-field geometry: full navpage editor on-device and via LX Styler (navboxes + symbols, colours, fonts, page count/modes, memory presets MEM1/MEM2).
- `●` Day / night / high-contrast modes: night mode + auto-brightness (ambient light sensor) and high/low-contrast terrain schemes.
- `●` Units: metric/imperial toggle, ballast as load or water, WGS84-vs-FAI-sphere distance method, CG as distance or moment.
- `●` Input / gestures / hardware buttons: 6/8 push-buttons + 4 rotary knobs, optional leather remote stick (assignable function buttons, PTT), optional multitouch.
- `●` Language / localization: 11 languages (Czech, German, English, Spanish, French, Italian, Dutch, Polish, Slovenian, Finnish, Russian), stored per profile.

### Additional features (outside the shared taxonomy)

- `●` **HAWK** inertial/aerodynamic air-data model: computes wind, netto/relative vario, sink-rate, angle-of-attack and sideslip from the inertial platform + polar (paid option); selectable as vario/SC audio source.
- `●` **AHRS / artificial horizon** + primary-flight-display symbols: artificial horizon with altitude, airspeed, vario and compass tapes; G-meter; flap tape with suggested-flap (wing-loading + g-factor).
- `●` **3D synthetic view**: 3D terrain with rivers/roads/airspace and FLARM traces.
- `●` **Weight & balance / CG-envelope** calculator (informational): arms/weights, permitted/caution/optimum CG zones, dry vs. wet CG, tail-ballast optimisation.
- `●` **Two-seat operation**: front/rear repeater sync of Mc/ballast/bugs, waypoint/airport/task, and wind (front-seat wind mode).
- `●` **Analog & telemetry inputs**: LXDAQ/LXDAQ+ 4-channel analog inputs and JRES engine/battery telemetry navboxes; pilot/co-pilot **heart-rate and SpO₂** navboxes.
- `●` **Checklists** and an on-device **PDF document viewer**.
- `●` **Simulator support**: free LXSim and Condor input (RS232, integrated in Condor 3) with real final-glide training.
- `●` **Radio/transponder & ACD57 multifunctional bridge** control from the navpages.
- `●` **Stopwatch**, **position/ATC report page** (magnetic radial + NM), and **potential-temperature** trigger aid.
- `●` **LXNAV Connect** cloud hub: automatic post-landing flight upload to multiple services/emails, profile/waypoint/map sync via Google Drive / Dropbox / SeeYou Cloud, weather-cache management.

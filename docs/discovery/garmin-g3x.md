# Discovery: Garmin G3X (non-touch)

> **Framing note:** The G3X is a general-aviation glass-cockpit EFIS for experimental / LSA aircraft, _not_ a soaring flight computer. Scored here against XCSoar's soaring taxonomy, most soaring-specific rows come out **absent** (MacCready, polar, thermal assistant, tasks/AAT, IGC, FLARM, WeGlide, live tracking), while a large body of powered-EFIS capability that has no row in this taxonomy is captured in §3.99. "Present via ADS-B/TIS/TAS" is called out explicitly wherever it stands in for a FLARM row, so the distinction isn't lost.

## System overview

- **Version(s) examined:** System Software v11.80 or later (Pilot's Guide 190-01115-00 Rev. Q)
- **Date(s) examined:** April 2019 (manual revision date)
- **Platform(s):** Dedicated Garmin hardware — GDU 37X / GDU 4XX sunlight-readable displays, knob/joystick/softkey driven (non-touch). Experimental & Light-Sport aircraft only; explicitly _not_ TSO-certified. No general-purpose OS.
- **License:** Proprietary (Garmin)
- **Offline behavior:** Fully self-contained offline — PFD, moving map, terrain, EIS, GPS nav, flight planning and databases all run from on-board SD-card data with no connectivity. Only datalink **weather** (SiriusXM / FIS-B) and **traffic** (ADS-B / TIS-A) require external radios and, for SiriusXM, a subscription.
- **Configuration model:** Two-tier. Installer-locked **Configuration Mode** sets aircraft type, sensors, Vg/sink-rate (drives the Glide Range Ring), autopilot servos, etc.; a user-level settings layer covers units, map setup, and data fields. No multi-pilot profile concept.

## Feature inventory

**Status legend:**

- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

### Map display & interaction

- `●` Vector / topographic map rendering: VFR / IFR / TOPO / Satellite base maps with aeronautical, geographic and topographic layers.
- `●` Terrain shading / elevation: topographic shading on the map (TOPO), plus a dedicated Terrain page with color-coding relative to aircraft altitude.
- `●` Map orientation (track / north / target up): North Up, Track Up, and Desired-Track Up (DTK Up), with a "North Up Above" range threshold.
- `◐` Auto-zoom & circling zoom (mode-dependent): Auto Zoom to the smallest range showing the active waypoint; no soaring circling-zoom mode.
- `●` Manual pan & zoom: FMS-joystick Map Pointer pan plus range zoom.
- `●` Snail trail / flight trail: electronic breadcrumb "track log" drawn on the map, distance- or time-interval based, with wrap.
- `●` Glide range ("reachability") line: **Glide Range Ring** — cyan reachable-area ring from pilot-entered Best Glide Speed + sink rate, corrected for AGL height, wind and bank; fades in above 500 ft AGL, with off-profile arrows. Engine-out safety tool, _not_ a MacCready soaring reachability line.
- `○` Thermal markers on map.
- `◐` Markers / pilot events on map: user waypoints can be created (including at present position), which serves as an ad-hoc marker; no dedicated in-flight event marker.
- `●` Waypoint & landable symbology / labels: airport / navaid / user-waypoint symbology, runway extension lines and runway numbers.
- `●` "What's here" query: Map Pointer over any feature (airport, METAR flag, airspace…) shows its details.

### Waypoints & navigation

- `●` Waypoint database: Jeppesen NavData (airports, VORs, NDBs, intersections) plus user waypoints.
- `◐` Landable vs non-landable distinction: airport DB carries surface type and public/private filters, but there is no glider outlanding-field / landable-terrain concept.
- `●` Nearest (waypoint / landable / airfield): NRST key → Nearest Airports / Airspace / etc., with a private-airport show/hide toggle.
- `●` Go-to / direct navigation: Direct-To.
- `●` Waypoint details (info, image, runway, freq): airport info with runways, frequencies, weather/METAR and SafeTaxi diagram. No photos, but geo-referenced diagrams/charts.
- `◐` Alternates / safety-landing selection: Nearest Airports plus the engine-out **Best Airport** bearing pointer / line approximate this; no glider alternate arrival-height list.

### Cross-country tasks

- `○` Racing tasks: has flight plans / routes, not soaring racing tasks.
- `○` Assign Area Tasks (AAT).
- `○` Cross-country task types (FAI triangle, out-and-return, DMSt, etc.).
- `○` Observation zones & start/finish/turnpoint rules.
- `◐` Task manager (build / edit): full flight-plan creation & editing (waypoint sequencing), but no soaring task / OZ semantics.
- `○` Task calculator (in-flight required speeds etc.).
- `◐` In-flight task edit: in-flight flight-plan editing is supported; soaring task editing is not.
- `○` FAI badge / record support.
- `◐` Task import / export / declaration to logger: flight plans import/export via SD card; no IGC task declaration to an external logger.

### Glide computer

- `○` Flight modes + auto display switching (cruise/circling/final): no soaring cruise/circling/final-glide display modes (AFCS auto-mode-switching is autopilot, not a glide computer).
- `○` MacCready setting (manual).
- `○` Auto MacCready (modes).
- `◐` Glide polar (library + custom): no polar model; a single Best-Glide-Speed + sink-rate pair (config-mode entry) drives the Glide Range Ring.
- `◐` Ballast / bugs / weight setup: Weight & Balance utility (CG / loading) exists; no soaring ballast/bugs feeding a polar.
- `○` Speed to fly / speed command / risk factor: airspeed-tape refs and VNE (optionally TAS-adjusted) only; no soaring STF.
- `◐` Safety heights / safety MacCready: en-route safe altitude / minimum safe altitude data fields plus the Glide Range Ring; no soaring safety-arrival-height config.
- `◐` Final glide calculator (wind, altitude required): VNAV computes required vertical speed / altitude to a waypoint or altitude constraint, and the Glide Range Ring shows the reachable area with wind — but neither is a soaring task final-glide.
- `○` Task speed estimation.
- `○` Optimal cruise track.

### Atmosphere & instruments

- `◐` Variometer display: standard VSI (vertical-speed indicator on the PFD), not a total-energy soaring vario.
- `○` Average climb: instantaneous VS and climb-gradient data fields only; no thermal-averaged climb.
- `○` Audio variometer: voice alerts and an AOA aural tone exist, but no soaring audio vario.
- `●` Air-data inputs (external sensors): GSU 25/73 ADAHRS (pitot/static), GTP 59 OAT probe, GAP 26 AOA/pitot.
- `●` Wind estimation (circling / zigzag / compass / external): continuous automatic air-data + GPS wind computation. No GPS-only circling/zigzag methods (unnecessary given true-airspeed sensing).
- `◐` Wind display & manual override: wind speed/direction or headwind/crosswind display on the PFD; no manual override (always computed).
- `○` Thermal locator / assistant / centering aid.
- `○` Thermal profile.
- `○` Convection forecast (cloud base etc.): no soaring convection product (some datalink cloud-tops data exists — see §3.6).

### Weather

- `●` METAR / TAF: text METAR/TAF via SiriusXM and FIS-B, plus graphical METAR flags on the map.
- `○` Forecast overlays (SkySight / TopMeteo / RASP): no soaring forecast providers.
- `●` Wind aloft / weather-station data: Winds & Temperatures Aloft and surface observations via datalink.
- `●` In-flight weather updates: live datalink weather — SiriusXM (subscription) or FIS-B (free, via ADS-B), with a broad US product suite (NEXRAD, echo tops, lightning, PIREPs, AIRMET/SIGMET, TFRs, etc.).

### Airspace

- `●` Airspace display (classes, filtering): airspace classes with altitude filtering / smart airspace.
- `●` Proximity / incursion warnings + acknowledgement: airspace alert messages on approach, acknowledgeable, with NRST-key jump to the Nearest Airspace page.
- `●` Airspace query / details: Nearest Airspace page and Map Pointer query.
- `◐` NOTAM handling: TFRs graphically via datalink and Jeppesen database-published NOTAMs (via ChartView); no full textual NOTAM briefing.

### Traffic & collision awareness (FLARM)

- `○` FLARM Traffic on map: no FLARM. (Traffic is shown via ADS-B / TIS-A / TAS — see §3.99.)
- `○` OGN Traffic on map.
- `○` FLARM and OGN Traffic deduplication.
- `○` FLARM radar view: no FLARM radar; ADS-B/TIS traffic is shown on the map and a traffic view with TCAS symbology.
- `○` FLARM traffic dead reckoning: (ADS-B/TIS targets coast, but this is not FLARM.)
- `●` Collision / proximity warnings: Traffic Advisories with TCAS symbology and voice callouts — **via ADS-B / TIS-A / TAS-TCAS, not FLARM.**
- `◐` Traffic details dialogue: selectable ADS-B/TIS targets show basic details.
- `○` Traffic ID / registration lookup: no registration/FlarmNet-style lookup.
- `○` Team flying / buddy codes.

### Avionics & airframe

- `●` Battery / voltage monitoring: main-bus voltmeter and battery-load (amperes) via the EIS.
- `●` GPS status / connection / altitude source: GPS Receiver Status page; GPS (geometric) altitude available.
- `●` Engine / powered flight (ENL, MoP, engine hours): full **powered-aircraft** EIS — RPM/tach, engine/tach hours, fuel flow & calculator, CHT/EGT, Lean Assist, CAS messages. (ENL/MoP are glider-motor concepts; the G3X does piston-engine monitoring instead.)
- `◐` Radio frequency management / control: optional auto-tuning that pushes frequencies to an external COM radio; no built-in radio.
- `◐` Transponder (XPDR) control / squawk: optional remote-transponder interface (code, mode, ident, ADS-B-Out transmit control).
- `●` Switch inputs (gear / flap warnings etc.): configurable warning inputs and CAS messages, plus Vertical Power (electronic circuit-breaker) integration.
- `●` Multiple external devices / slave mode: dual-display (PFD + MFD), external IFR navigators, ADS-B, ADAHRS, and autopilot servos on shared buses.

### Data fields (InfoBox system)

- `●` Configurable data-field grid: user-configurable Data Bar (4 fields) and Info-Page data fields, independently per display.
- `●` Multiple data-field pages / layouts: selectable Info-Page layouts and split-screen arrangements.
- `○` Per-flight-mode auto layout: no soaring mode-driven auto layout.
- `●` Altitude / altimetry values (QNH, pressure alt, FL, AGL): baro altitude (with altimeter setting), pressure/density altitude, AGL height.
- `●` Speed values (GS / TAS / IAS / optimal): IAS tape, TAS readout, GS field; no soaring "optimal speed".
- `●` Direction values (track / bearing / heading): TRK, BRG, DTK, HDG.
- `●` Time values (UTC / local / ETA / ETE / sunset / flight time): UTC, Local, ETA, ETE, Flight Time. (Sunrise/sunset not among the offered fields.)
- `○` Touch / gesture interaction with data fields: non-touch platform; selection is via knob/joystick/softkeys.

### Analysis & review

- `○` Barograph / altitude trace: no in-app barograph (the CSV flight-data log can be analyzed off-board).
- `○` Climb history / thermal analysis.
- `○` Wind analysis: live wind only.
- `○` Glide polar analysis.
- `○` Task analysis.
- `○` OLC / contest analysis.
- `◐` Airspace cross-section: the Terrain page has a vertical **profile view** (terrain cross-section along track); airspace is not shown in cross-section.

### Contest / WeGlide optimization (live)

- `○` Live WeGlide optimization in-flight.
- `○` Flight trace maintenance: (a breadcrumb track log exists, but not for contest optimization).
- `○` Live scoring / achieved distance.

### Live tracking

- `○` Live tracking upload (OGN / SkyLines / cloud): no live tracking upload.
- `○` Retrieve / crew comms / position sharing.

### Instrument & device connectivity (I/O)

- `○` Bluetooth: for the non-touch G3X, datalink devices connect via RS-232 serial. (The manual carries some G3X _Touch_ text mentioning Bluetooth, which does not apply to this platform.)
- `○` BLE.
- `●` Serial: RS-232 serial is the primary bus — external navigators, ADS-B, transponder, MapMX map-data input.
- `○` USB: data exchange is via SD card, not USB.
- `○` TCP/IP / network.
- `○` NMEA input parsing: uses aviation serial formats (MapMX, Garmin/ARINC via external navigators), not glider-style NMEA 0183.
- `○` NMEA / data output (drive other devices): no documented NMEA output; autopilot is driven over Garmin's own protocol.
- `●` Positioning source selection (internal / external): internal GPS or an external WAAS IFR navigator can be selected as the position source.
- `◐` Vendor protocols (LXNav / FLARM / …): Garmin/aviation ecosystem only (GDL ADS-B, GTX transponder, GSU ADAHRS, external Garmin navigators); no LXNav/FLARM soaring vendor protocols.

### Data & file management

- `◐` Waypoint files (CUP / …): user waypoints import/export via SD card in Garmin format; no SeeYou CUP.
- `○` Airspace files (OpenAir / CUB / …): airspace comes from the Jeppesen aviation database, not user OpenAir files.
- `●` Terrain / topology / map data: terrain, obstacle, SafeTaxi, basemap and chart databases on SD card.
- `◐` In-app data download & updates: databases are updated by loading SD cards prepared on a PC (Garmin), not fully in-app / OTA.
- `◐` Aircraft / plane profiles (polar, ballast, reg, comp ID): a single per-aircraft configuration (type, Vg/sink, W&B); no swappable multi-aircraft profiles, no polar/competition ID.
- `○` Handicap / polar lists.

### Logging & recording

- `○` IGC flight log recording: records 1 Hz CSV flight + engine data to SD card, openable in a spreadsheet — not IGC.
- `○` Approved / signed logger (badge / record): not an IGC-approved flight recorder.
- `○` Flight replay: Flight Log (list of up to 50 flights with date/route/time) and a track-log trail exist, but no replay animation.
- `◐` Pilot events / markers logging: user waypoints can be dropped; no dedicated pilot-event/marker log stream.

### Configuration & UI

- `◐` Configuration profiles (per user / per aircraft / global): installer Configuration Mode (per aircraft) plus a user-settings layer; no per-user pilot profiles.
- `●` Screen layout / data-field geometry: selectable page layouts, split-screen, and configurable data fields.
- `◐` Day / night / high-contrast modes: Auto or Manual backlight (Auto tracks the aircraft lighting-bus voltage); no dedicated day/night color theme.
- `●` Units: Units Setup page — distance/speed, altitude, temperature, pressure, and nav-angle (magnetic/true).
- `●` Input / gestures / hardware buttons: knob (FMS joystick), softkeys and dedicated hardware keys (non-touch).
- `○` Language / localization: no language selection documented (English only).

### Additional features (outside the shared taxonomy)

Powered-EFIS capability with no row above — this is where the bulk of the G3X actually lives:

- `●` Primary Flight Display: attitude, airspeed, altitude, HSI/course deviation, slip/skid, from ADAHRS.
- `●` Synthetic Vision (SVX): 3D forward-looking terrain/obstacle/traffic/runway rendering with flight-path marker.
- `●` Engine Indication System (EIS): full engine/fuel monitoring, Lean Assist mode, Fuel Calculator, CAS messages.
- `●` Automatic Flight Control System (autopilot): flight director, lateral & vertical modes, IAS/VS/ALT/VNAV, coupled GPS/VOR/LOC/BC and ILS approaches (with external navigator), Go-Around/Takeoff modes.
- `●` Electronic Stability & Protection (ESP-X): attitude-based envelope protection via Garmin servos.
- `●` Angle of Attack (AOA): visual gauge plus escalating aural alert (with GAP 26 + GSU 25).
- `●` VNAV: vertical navigation to altitude constraints with target-altitude capture.
- `●` ADS-B In/Out + TIS-A + TAS/TCAS traffic: TCAS symbology, voice Traffic Advisories.
- `●` Datalink weather: SiriusXM + FIS-B full US product suite.
- `●` Terrain Proximity alerting: color-coded terrain plus vertical profile view.
- `●` SafeTaxi: geo-referenced airport surface diagrams with hot-spot depiction.
- `●` ChartView (Jeppesen) / FliteCharts: geo-referenced approach plates and airport charts.
- `●` Airport Directory: AOPA/AC-U-KWIK directory data.
- `●` SiriusXM Radio: audio entertainment integration.
- `●` Electronic checklists: SD-card checklist files.
- `●` Carbon Monoxide detector & Pulse Oximeter integration.
- `●` Vertical Power (VP-X): electronic circuit-breaker / electrical-system control.
- `●` Weight & Balance utility: CG and loading computation.
- `●` IFR GPS navigation: airways, holds, procedure turns and approaches via an external WAAS navigator.

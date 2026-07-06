# Discovery: WeGlide Copilot

## System overview

- **Version(s) examined:** 1.46.0 (map data current to 2026-04-20)
- **Date(s) examined:** 2026-07-06
- **Platform(s):** Android (`org.weglide.copilot`), iOS (App Store id 6478563635), and Web/PWA (`copilot.weglide.org`). Single codebase: a Vue 3 web app packaged with **Capacitor** for the native builds.
- **License:** Proprietary (WeGlide). Freemium — free core with paid tiers ("WeGlide Premium" + "SkySight" add-on) via native in-app purchase and Paddle web checkout, 14-day trial.
- **Offline behavior:** Minimal. The PWA shell is cached, so the app opens offline, and **IGC recording runs fully offline** (internal GPS + barometer, stored locally, uploaded later). All substantive content — vector map tiles, weather/satellite/forecast layers, live traffic and live thermals — requires connectivity. Marketing copy stresses it is "optimized for low internet," not that it works without it.
- **Configuration model:** Very light. One WeGlide account; aircraft chosen by registration/class from the WeGlide fleet. Settings cover units, font size, map orientation, ENL on/off, and which map layers are shown. There are no per-aircraft/per-pilot profiles, no polar/ballast configuration, and no InfoBox layout editor.

**Architecture notes.** The frontend is a Vue 3 single-page web app rendering vector maps with **MapLibre GL JS** over **Protomaps** tiles served by WeGlide (base map, airports, airspace, outlanding fields) plus a WeGlide terrain source (contours + hypsometric shading). The iOS/Android builds are the same web app wrapped with **Capacitor**. Positioning uses the device's internal GPS and barometer only; there is no Bluetooth, BLE, USB, or serial pairing anywhere in the app, so it cannot talk to any external avionics. Backend services the app talks to: `api.weglide.org` and `live.weglide.org`, SkySight (`edge/rain/satellite.skysight.io`), RainViewer, a WeGlide live-fix/thermal CDN, OGN (`glidernet.org` DDB), openAIP, SoaringSpot, and streckenflug.at.

## Feature inventory

**Status legend:**
- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

### Map display & interaction
- `●` Vector / topographic map rendering: MapLibre GL + Protomaps vector base with airports, airspace, landout fields, peaks, water, places.
- `●` Terrain shading / elevation: hypsometric shaded relief + contour lines (major/minor) with labels + mountain peaks, from WeGlide terrain data.
- `◐` Map orientation (track / north / target up): only **North-up** and **Heading-up**. No target-up.
- `○` Auto-zoom & circling zoom (mode-dependent): no auto-zoom or circling-zoom logic.
- `●` Manual pan & zoom.
- `●` Snail trail / flight trail: own track rendered.
- `○` Glide range ("reachability") line: no reachability/glide-range computation.
- `●` Thermal markers on map: live thermals and historic thermal hotspots.
- `○` Markers / pilot events on map: the selected-map-object panel is generic, not user-placed markers or pilot events.
- `●` Waypoint & landable symbology / labels: waypoints (with hit-box), color-coded outlanding fields (green/yellow/red), airports, all labelled.
- `●` "What's here" query: tap opens panels for airspace ("Airspaces here"), airport, waypoint, thermal, and aircraft.

### Waypoints & navigation
- `●` Waypoint database: server-provided waypoints/airports with search.
- `●` Landable vs non-landable distinction: dedicated outlanding-field layer (green/yellow/red quality) separate from airports and waypoints.
- `○` Nearest (waypoint / landable / airfield): no "nearest" function.
- `●` Go-to / direct navigation: draws a direct line/bearing to a target with "Stop navigation".
- `●` Waypoint details (info, image, runway, freq): the airport panel shows runway, frequency, altitude, class.
- `○` Alternates / safety-landing selection: no alternate or safety-landing logic (no glide computer).

### Cross-country tasks
- `◐` Racing tasks: racing-style task kinds are displayed, but there is no in-flight racing task computer.
- `◐` Assign Area Tasks (AAT): the AA/AAT kind is recognized and drawn, but no AAT area optimization/calculator.
- `●` Cross-country task types: task kinds cover DMSt, FAI, Triangle, Out & Return, Multi Triangle, Multi Rectangle, Rectangle, 3 TP, Goal, Grand Prix, Poland, US, UK, Free (display level).
- `◐` Observation zones & start/finish/turnpoint rules: task sectors are rendered, but zone rules/scoring are not evaluated on-device.
- `◐` Task manager (build / edit): tasks are **imported/selected** from WeGlide and SoaringSpot, not built or edited from scratch on the device.
- `○` Task calculator (in-flight required speeds etc.): none.
- `○` In-flight task edit: none.
- `○` FAI badge / record support: none (signs IGC but is not FAI-certified — see 3.16).
- `◐` Task import / export / declaration to logger: import from WeGlide/SoaringSpot and **declaration to the internal recorder** ("Task declared" / "Failed to declare task"); no export to an external logger.

### Glide computer
- `○` Flight modes + auto display switching (cruise/circling/final): no cruise/circling/final display modes (an internal thermal-detection state exists for thermal detection only).
- `○` MacCready setting (manual): none.
- `○` Auto MacCready (modes): none.
- `○` Glide polar (library + custom): none.
- `○` Ballast / bugs / weight setup: none.
- `○` Speed to fly / speed command / risk factor: none.
- `○` Safety heights / safety MacCready: none.
- `○` Final glide calculator (wind, altitude required): none.
- `○` Task speed estimation: none (average speed is shown for *tracked* gliders — see 3.12 — not a task calc).
- `○` Optimal cruise track: none.

### Atmosphere & instruments
- `◐` Variometer display: a "Vario (10s)" value (10-second averaged climb) is shown for own ship and live aircraft, derived from GPS/barometer; no needle/gauge instrument.
- `●` Average climb: averaged climb is a first-class value (thermal "average", Vario 10s).
- `○` Audio variometer: none (no tone/beep).
- `○` Air-data inputs (external sensors): only the internal phone barometer (pressure altitude) + GPS; no external air data.
- `○` Wind estimation (circling / zigzag / compass / external): no on-board wind estimation; forecast wind comes from SkySight.
- `◐` Wind display & manual override: forecast/hotspot wind is shown (8-way N/NE/…/NW) and the user picks a wind-direction filter; there is no measured-wind override.
- `○` Thermal locator / assistant / centering aid: thermals/hotspots are shown on the map, but there is no live centering assistant.
- `◐` Thermal profile: "Height of Thermals" layer + an altitude-band selection give a coarse thermal-height-by-altitude picture.
- `●` Convection forecast (cloud base etc.): SkySight `Cloudbase`, `Thermals`, `Height of Thermals`, `Vertical Airspeed`, `XC Speed`, `Convergence`, `Wave`.

### Weather
- `○` METAR / TAF: none.
- `●` Forecast overlays (SkySight / TopMeteo / RASP): deep **SkySight** integration (convergence, XC speed, wave, cloudbase, thermals, height of thermals, vertical airspeed), gated behind a subscription tier.
- `◐` Wind aloft / weather-station data: SkySight forecast winds selectable by altitude band; no live station data.
- `●` In-flight weather updates: live weather satellite (with "Sat image updated" toast), rain radar (RainViewer + SkySight rain), and live thermals/hotspots refresh in flight.

### Airspace
- `●` Airspace display (classes, filtering): classes A/B/C/D, control zone, danger, restricted, gliding, mandatory zone, FIS, special — with per-class styling and an altitude-band filter.
- `◐` Proximity / incursion warnings + acknowledgement: "Airspace Crossings" scans the track for violations ("Scanning for airspace violations…"); this is crossing detection/review rather than a real-time proximity alert with acknowledge.
- `●` Airspace query / details: an airspace panel + "Airspaces here" + airspace information ("may contain errors" disclaimer).
- `○` NOTAM handling: none.

### Traffic & collision awareness (FLARM)
- `○` FLARM Traffic on map: no FLARM device link possible (no BT/BLE).
- `●` OGN Traffic on map: live traffic of other gliders from WeGlide live / OGN network.
- `○` FLARM and OGN Traffic deduplication: n/a (no FLARM source).
- `○` FLARM radar view: none.
- `○` FLARM traffic dead reckoning: none.
- `○` Collision / proximity warnings: none.
- `●` Traffic details dialogue: the aircraft panel shows registration, class, pilot, speed, Alt. MSL, Vario (10s), XC distance.
- `●` Traffic ID / registration lookup: OGN DDB lookup + aircraft-registration search ("No aircraft found in the OGN database").
- `◐` Team flying / buddy codes: aircraft can be **marked/unmarked** and live-tracking filtered by airport, but there are no FLARM buddy codes.

### Avionics & airframe
- `○` Battery / voltage monitoring: none.
- `◐` GPS status / connection / altitude source: geolocation status/error handling and dual altitude source (GPS + barometer pressure altitude); no satellite-level GPS status.
- `●` Engine / powered flight (ENL, MoP, engine hours): **ENL (engine-noise-level) recording via the microphone** ("Record ENL"), written into the IGC.
- `○` Radio frequency management / control: airport frequency is displayed read-only; no control.
- `○` Transponder (XPDR) control / squawk: none.
- `○` Switch inputs (gear / flap warnings etc.): none.
- `○` Multiple external devices / slave mode: none.

### Data fields (InfoBox system)
- `○` Configurable data-field grid: no InfoBox system; the UI is map-centric with fixed pills and slide-up cards.
- `○` Multiple data-field pages / layouts: none.
- `○` Per-flight-mode auto layout: none.
- `◐` Altitude / altimetry values (QNH, pressure alt, FL, AGL): "Altitude" / "Alt. MSL" shown; barometer supplies pressure altitude; no QNH/FL/AGL selection.
- `◐` Speed values (GS / TAS / IAS / optimal): ground speed shown; no TAS/IAS (no air data).
- `◐` Direction values (track / bearing / heading): bearing to GoTo target + map heading; compass heading via device orientation.
- `◐` Time values (UTC / local / ETA / ETE / sunset / flight time): flight duration, takeoff/landing times, "active for", "last updated"; no ETA/ETE/sunset.
- `◐` Touch / gesture interaction with data fields: pills and slide-ups are tappable, but there is no data-field grid to interact with.

### Analysis & review
- `○` Barograph / altitude trace: none.
- `○` Climb history / thermal analysis: none in-app.
- `○` Wind analysis: none.
- `○` Glide polar analysis: none.
- `○` Task analysis: none in-app.
- `◐` OLC / contest analysis: the post-flight analysis view shows takeoff/landing/duration/date + airspace crossings; full scoring/optimization happens after upload to WeGlide, not on-device.
- `○` Airspace cross-section: a crossings *list* exists, but no vertical cross-section view.

### Contest / WeGlide optimization (live)
- `◐` Live WeGlide optimization in-flight: live tracked gliders show "XC Distanz" and a ranking; the optimization itself is server-side.
- `●` Flight trace maintenance: continuous track recording and live-track maintenance.
- `●` Live scoring / achieved distance: achieved XC distance and live ranking are displayed.

### Live tracking
- `●` Live tracking upload (OGN / SkyLines / cloud): own position published to WeGlide live tracking (+ OGN); LiveConnect improves position data and makes the pilot appear by name for ~18 h.
- `◐` Retrieve / crew comms / position sharing: system share plus live view of others and "mark aircraft"; no dedicated retrieve/crew-chat feature.

### Instrument & device connectivity (I/O)
- `○` Bluetooth: not present.
- `○` BLE: not present.
- `○` Serial: not present.
- `○` USB: not present.
- `●` TCP/IP / network: all data over HTTPS to WeGlide/SkySight/CDN APIs.
- `○` NMEA input parsing: none.
- `○` NMEA / data output (drive other devices): none.
- `○` Positioning source selection (internal / external): internal GPS + barometer only; no external source selection.
- `○` Vendor protocols (LXNav / FLARM / …): none.

### Data & file management
- `○` Waypoint files (CUP / …): waypoints come from the WeGlide server; no CUP import.
- `○` Airspace files (OpenAir / CUB / …): airspace comes from the server (openAIP-sourced); no OpenAir import.
- `●` Terrain / topology / map data: server vector tiles (Protomaps) + terrain/hypsometric, cached client-side.
- `◐` In-app data download & updates: tiles/data are fetched and cached on demand; there is no explicit "download this region for offline" manager.
- `◐` Aircraft / plane profiles (polar, ballast, reg, comp ID): aircraft selected by registration/class from the WeGlide fleet; no polar/ballast/comp-ID profile.
- `○` Handicap / polar lists: none.

### Logging & recording
- `●` IGC flight log recording: records IGC from internal GPS + barometer (+ optional ENL); sessions and a logbook are kept locally.
- `◐` Approved / signed logger (badge / record): the IGC is **cryptographically signed** (ECDSA) and backed by device attestation for WeGlide's own verification — but it is **not an FAI-approved flight recorder** for badges/records.
- `○` Flight replay: recorded flights are viewable on a map (static track), but there is no time-based replay.
- `○` Pilot events / markers logging: no PEV/marker logging into the IGC.

### Configuration & UI
- `○` Configuration profiles (per user / per aircraft / global): single account, no profiles.
- `○` Screen layout / data-field geometry: fixed map-centric layout (pills + slide-ups); only layer visibility and font size are adjustable.
- `◐` Day / night / high-contrast modes: the app ships a dark UI theme; no explicit user day/night map toggle observed.
- `●` Units: metric and imperial (m/ft; km/h, kn, mph; m/s).
- `◐` Input / gestures / hardware buttons: touch/gestures on map and controls; no hardware-button mapping.
- `●` Language / localization: full English + German localization, selected by device locale.

### Additional features (outside the shared taxonomy)

- `●` SkySight forecast suite as premium layers: Convergence ("energy lines"), XC Speed, Wave, Cloudbase, Height of Thermals, Vertical Airspeed, Thermals.
- `●` Live weather-satellite imagery with a time slider.
- `●` Rain radar overlay (RainViewer + SkySight rain).
- `●` Thermal hotspots (historic database) plus crowd-sourced live thermals derived from live fixes.
- `●` Live glider network with names/ranking (WeGlide live + OGN) and OGN DDB registration lookup.
- `●` Freemium monetization: native in-app purchases + Paddle web checkout, "WeGlide Premium" and "SkySight" tiers, 14-day trial, restore purchases.
- `●` Security/identity: device attestation and biometric login.
- `●` One-tap upload of the recorded flight to WeGlide (Export IGC / Export JSON / Upload / View on WeGlide).

# Discovery: LX Navigation Navia

> **Key caveat that colours the whole inventory:** Navia is a very new, actively
> launching ecosystem. LX navigation has **not published a pilot / user manual for
> Navia OS** — only an *installation* manual and a catalogue. As a result, the
> hardware, connectivity, sensor, traffic, display, audio and cloud layers are
> reasonably well documented, but the classic soaring **flight‑computer software
> behaviour** (MacCready logic, task engine, final glide, thermal assistant,
> InfoBox catalogue, analysis pages) is essentially undocumented in public sources.
> Those items are marked `?` on purpose rather than inferred from the heritage
> LX Zeus/Era/Eos line, even where heritage makes them likely.

## System overview

- **Version(s) examined:** No unit/firmware examined. Documentation basis: Navia Core Pro Installation Manual **R2** (2026‑05‑20); Navia Catalogue **2025 v1.1**. Navia OS software version not stated in public materials; product is described as shipping / special‑order (~4 week lead time).
- **Date(s) examined:** 2026‑07‑06
- **Platform(s):** Proprietary embedded avionics, not a phone/PC/tablet app. **Navia Core Pro** is the central compute unit ("the brain"); **Navia OS** is the UI, rendered on **Navia Displays** (4″ / 7″ / 12″), touch‑first. Landscape or portrait; 80 mm (3.125″) standard instrument mount. Multiple displays and multi‑seat (dual primary display + dual Grip) supported. OS internals not disclosed (LX's previous LX9000 generation ran Linux; not confirmed for Navia).
- **License:** Proprietary, closed hardware + software. Marketed as "open‑source hardware and documented communication protocols" for third‑party integration, but the product itself is commercial. Functionality is **license‑gated** (e.g. *Core Pro IGC license*, *ENL license*, *Navia Sense Gliding Pro / Pro+ license*) on top of the base hardware.
- **Offline behavior:** The AI voice assistant is explicitly **offline** (on‑device recognition). Core map/nav is presumed on‑device (vector map, airspace, terrain). Cloud sync, live weather, weather forecast overlays and live tracking depend on the **built‑in Global SIM / LTE** connectivity and LX Cloud, i.e. they need a data link.
- **Configuration model:** Web **Configurator** (configurator.lxnavigation.com) + **LX Cloud** account (cloud.lxnavigation.com) for setup/management, plus on‑device customizable widget layouts (position, NavBar items, typography size) and license activation. Per‑aircraft / per‑pilot profile depth not documented.

## Feature inventory

**Status legend:**
- `●` **full** — present and works as a first-class capability
- `◐` **partial** — present but limited, awkward, or incomplete
- `○` **absent** — not present
- `?` **unknown** — could not be determined from available sources

> In this document `?` dominates the flight‑computer sections **not** because the
> features are absent, but because no user manual exists to confirm them. Where a
> feature is advertised or clearly implied, that is stated in the note.

### Map display & interaction
- `●` Vector / topographic map rendering: Navia OS lets the pilot select a **Vector** basemap (also Satellite and sectional VFR charts).
- `?` Terrain shading / elevation: Not documented. Satellite imagery basemap exists; explicit terrain/elevation shading of the vector map is unconfirmed.
- `?` Map orientation (track / north / target up): Not documented.
- `?` Auto-zoom & circling zoom (mode-dependent): Not documented.
- `●` Manual pan & zoom: Navia OS is "touch‑first," described as the only avionics fully controllable by touch alone; direct map manipulation is a core interaction.
- `?` Snail trail / flight trail: Not documented (likely, but unconfirmed).
- `?` Glide range ("reachability") line: Not documented.
- `?` Thermal markers on map: Not documented.
- `?` Markers / pilot events on map: Not documented.
- `◐` Waypoint & landable symbology / labels: Selectable **Airports** and **POIs** map layers exist; glider‑specific landable classification/symbology is unconfirmed.
- `?` "What's here" query: Not documented.

### Waypoints & navigation
- `◐` Waypoint database: Airports + POIs layers and a "Flight plan" concept are documented; a dedicated glider waypoint database and its format are unconfirmed.
- `?` Landable vs non-landable distinction: Not documented.
- `?` Nearest (waypoint / landable / airfield): Not documented.
- `◐` Go-to / direct navigation: AI voice can "set waypoints" and "activate navigation modes"; a direct‑to capability is implied but the UI/behaviour is undocumented.
- `?` Waypoint details (info, image, runway, freq): Not documented.
- `?` Alternates / safety-landing selection: Not documented.

### Cross-country tasks
> Navia OS is marketed as a single system for "cross country, glider, competition, VFR, IFR." That positioning implies a task engine, but none of the specifics below are documented in public materials.
- `?` Racing tasks: Not documented.
- `?` Assign Area Tasks (AAT): Not documented.
- `?` Cross-country task types (FAI triangle, out-and-return, DMSt, etc.): Not documented; only a generic "Flight plan" is mentioned.
- `?` Observation zones & start/finish/turnpoint rules: Not documented.
- `◐` Task manager (build / edit): A "Flight plan" layer/editing concept exists; a glider‑grade task manager is unconfirmed.
- `?` Task calculator (in-flight required speeds etc.): Not documented.
- `?` In-flight task edit: Not documented.
- `?` FAI badge / record support: Not documented.
- `?` Task import / export / declaration to logger: Not documented (an IGC logger exists via license; task declaration is unconfirmed).

### Glide computer
> Entirely undocumented in public sources. The heritage LX Zeus + Era/Eos systems provided the full MacCready/speed‑to‑fly/final‑glide suite, so presence is plausible, but Navia‑specific behaviour cannot be confirmed.
- `?` Flight modes + auto display switching (cruise/circling/final): Not documented.
- `?` MacCready setting (manual): Not documented.
- `?` Auto MacCready (modes): Not documented.
- `?` Glide polar (library + custom): Not documented.
- `?` Ballast / bugs / weight setup: Not documented.
- `?` Speed to fly / speed command / risk factor: Not documented (a core vario function; unconfirmed for Navia).
- `?` Safety heights / safety MacCready: Not documented.
- `?` Final glide calculator (wind, altitude required): Not documented.
- `?` Task speed estimation: Not documented.
- `?` Optimal cruise track: Not documented.

### Atmosphere & instruments
- `●` Variometer display: Vario is a first‑class widget; a dedicated **Navia Vario Indicator** (traditional needle instrument) is offered, and the **Navia Sense** sensor unit provides vario data. An "inertial variometer" is enabled by the Sense Gliding Pro/Pro+ license.
- `?` Average climb: Not explicitly documented (standard vario feature; unconfirmed).
- `◐` Audio variometer: The system has audio output including "spatial audio" (3D soundscape, up to 4 speakers). A conventional audio vario tone is expected but not explicitly documented as such.
- `●` Air-data inputs (external sensors): **Navia Sense** measures air data (pitot/static), GNSS and AHRS; multiple air‑data variants exist (glider basic/Pro+, powered with AoA, high‑speed to 500 kts).
- `●` Wind estimation (circling / zigzag / compass / external): **"Instant wind" / inertial wind** computed by Navia Sense (Gliding Pro/Pro+ license "Activated Inertial Wind and Vario calculations").
- `◐` Wind display & manual override: Wind is computed and displayable (widget); manual override is unconfirmed.
- `?` Thermal locator / assistant / centering aid: Not documented.
- `?` Thermal profile: Not documented.
- `?` Convection forecast (cloud base etc.): Not documented (live weather/forecast layers exist; cloudbase‑specific output unconfirmed).

### Weather
- `◐` METAR / TAF: AI voice can "request real‑time weather"; a live weather layer + LTE exist. METAR/TAF specifically is unconfirmed.
- `◐` Forecast overlays (SkySight / TopMeteo / RASP): A "weather forecast" map layer is documented; the specific provider(s) are not named.
- `?` Wind aloft / weather-station data: Not documented.
- `●` In-flight weather updates: Built‑in Global SIM/LTE + live‑weather layer are marketed for in‑flight, connected weather.

### Airspace
- `●` Airspace display (classes, filtering): "Airspace" is a selectable map layer in Navia OS.
- `?` Proximity / incursion warnings + acknowledgement: Not documented.
- `?` Airspace query / details: Not documented.
- `?` NOTAM handling: Not documented.

### Traffic & collision awareness (FLARM)
- `●` FLARM Traffic on map: **Navia Traffic** provides FLARM; a "traffic" map layer is documented. Available as *Flarm Only* or *Dual Flarm & ADS‑B* (ADS‑B‑in).
- `?` OGN Traffic on map: Not documented (LTE/cloud could enable it, but unconfirmed).
- `?` FLARM and OGN Traffic deduplication: Not documented.
- `?` FLARM radar view: Not documented (likely, but unconfirmed).
- `?` FLARM traffic dead reckoning: Not documented.
- `●` Collision / proximity warnings: Navia Traffic is marketed as a "collision avoidance system"; Navia Grip also gives haptic alerts for critical situations.
- `?` Traffic details dialogue: Not documented.
- `?` Traffic ID / registration lookup: Not documented.
- `?` Team flying / buddy codes: Not documented.

### Avionics & airframe
- `◐` Battery / voltage monitoring: **Navia UPS** provides backup/uninterruptible power; a voltage/battery widget is expected but not explicitly documented.
- `◐` GPS status / connection / altitude source: GNSS is present in Navia Sense and Navia Traffic (external GPS antenna required); a status/quality display is expected but not documented.
- `◐` Engine / powered flight (ENL, MoP, engine hours): An **ENL license** is offered (records engine noise in IGC via Navia Traffic); Navia MOP / EMU devices target powered/self‑launch/sustainer setups. MoP logging and engine‑hours display are unconfirmed.
- `◐` Radio frequency management / control: AI voice tunes the radio ("set radio one one niner decimal four seven five") and selects pre‑programmed channels; requires a compatible connected radio.
- `◐` Transponder (XPDR) control / squawk: AI voice can "set transponder code and mode"; requires a compatible connected transponder.
- `?` Switch inputs (gear / flap warnings etc.): Not documented.
- `●` Multiple external devices / slave mode: The whole architecture is a networked multi‑device system — Navia Hub links Core Pro to multiple displays / Emu / Mop; two‑seaters run dual primary displays and dual Grips.

### Data fields (InfoBox system)
- `●` Configurable data-field grid: Pilots customize which "widgets" are shown, their position, NavBar items and typography size.
- `●` Multiple data-field pages / layouts: e.g. Navia Display 4 lets the pilot scroll through configured widget sets; any number of displays can be attached, each configurable.
- `?` Per-flight-mode auto layout: Not documented.
- `◐` Altitude / altimetry values (QNH, pressure alt, FL, AGL): An altitude widget exists (air‑data sourced); the specific altimetry variants are unconfirmed.
- `◐` Speed values (GS / TAS / IAS / optimal): Airspeed widget + air‑data (high‑accuracy IAS, and TAS via air data); "optimal speed" is unconfirmed.
- `◐` Direction values (track / bearing / heading): AHRS + GNSS provide heading/track; widgets exist. Specific bearing‑to‑target fields unconfirmed.
- `?` Time values (UTC / local / ETA / ETE / sunset / flight time): Not documented.
- `●` Touch / gesture interaction with data fields: Touch‑first UI; widgets are scrollable/movable by touch. Navia Grip adds hardware HMI.

### Analysis & review
> No on‑device analysis pages are documented. LX Cloud may cover some post‑flight analysis, but this is unconfirmed.
- `?` Barograph / altitude trace: Not documented.
- `?` Climb history / thermal analysis: Not documented.
- `?` Wind analysis: Not documented.
- `?` Glide polar analysis: Not documented.
- `?` Task analysis: Not documented.
- `?` OLC / contest analysis: Not documented (possibly via LX Cloud; unconfirmed).
- `?` Airspace cross-section: Not documented.

### Contest / WeGlide optimization (live)
- `?` Live WeGlide optimization in-flight: Not documented; no WeGlide integration mentioned.
- `?` Flight trace maintenance: Not documented.
- `?` Live scoring / achieved distance: Not documented.

### Live tracking
- `●` Live tracking upload (OGN / SkyLines / cloud): Built‑in Global SIM/LTE (two LTE antennas) + LX Cloud provide connected live tracking. OGN/SkyLines as specific targets are unconfirmed; LX Cloud is the documented destination.
- `◐` Retrieve / crew comms / position sharing: Cloud‑based position sharing is implied by the "Aviation Command Center" cloud; a dedicated retrieve/crew workflow is unconfirmed.

### Instrument & device connectivity (I/O)
- `?` Bluetooth: Not documented.
- `?` BLE: Not documented.
- `●` Serial: RS‑232 wiring is documented in the installation manual (e.g. Air Avionics ACD RS232‑RX on the Core Pro connector).
- `?` USB: Not documented.
- `●` TCP/IP / network: Gigabit Ethernet is the system backbone (PoE, 1 Gbps between Navia components via Navia Hub).
- `◐` NMEA input parsing: Integrates FLARM and external air data; NMEA‑style serial input is implied by RS‑232 device integration but not spelled out.
- `◐` NMEA / data output (drive other devices): "Documented communication protocols" are advertised for driving/integrating other devices; the exact output sentences are not published.
- `◐` Positioning source selection (internal / external): GNSS exists in both Navia Sense and Navia Traffic (plus a dedicated GPS antenna); explicit source‑selection UI is unconfirmed.
- `●` Vendor protocols (LXNav / FLARM / …): Native FLARM support; documented integration with third‑party avionics (e.g. Air Avionics ACD) in the installation manual; LX's own protocol ecosystem.

### Data & file management
- `?` Waypoint files (CUP / …): Not documented.
- `?` Airspace files (OpenAir / CUB / …): Airspace layer exists, but user file import/format is not documented.
- `◐` Terrain / topology / map data: Vector, satellite and sectional VFR basemaps are provided; data provisioning is presumably via LX Cloud but the model is not detailed.
- `◐` In-app data download & updates: LX Cloud + built‑in LTE support connected updates; the exact on‑device update flow is unconfirmed.
- `?` Aircraft / plane profiles (polar, ballast, reg, comp ID): Not documented.
- `?` Handicap / polar lists: Not documented.

### Logging & recording
- `●` IGC flight log recording: Offered via license — *Core Pro IGC license* and/or *Navia Traffic IGC license*.
- `◐` Approved / signed logger (badge / record): An IGC logger is sold, but the formal IGC approval level (badge/record signing) is not stated in public materials.
- `?` Flight replay: Not documented.
- `◐` Pilot events / markers logging: An **ENL license** adds engine‑noise recording to IGC; explicit pilot‑event/marker logging is unconfirmed.

### Configuration & UI
- `◐` Configuration profiles (per user / per aircraft / global): LX Cloud + web Configurator manage setup; per‑aircraft/per‑pilot profile depth is unconfirmed.
- `●` Screen layout / data-field geometry: Fully customizable widget layout — position, NavBar, typography size, multiple displays.
- `◐` Day / night / high-contrast modes: Displays reach 2500 nits with Full Array Local Dimming and are marketed as readable "from brightest sunshine to darkest night," with ambient‑light adaptation; an explicit night/high‑contrast UI theme is unconfirmed.
- `?` Units: Not documented (configurable units expected but unconfirmed).
- `●` Input / gestures / hardware buttons: Touch‑first, plus **Navia Grip** HMI (buttons, haptic/vibration alerts, heated grip, voice‑assistant trigger) and rotary/knob input on the Vario Indicator.
- `?` Language / localization: AI voice is English; UI localization is not documented.

### Additional features (outside the shared taxonomy)

- `●` **AI voice assistant** — offline, aviation‑trained; sets waypoints/nav modes/flight plans, tunes radio, sets transponder code/mode, requests weather, and supports custom voice commands. Marketed as an aviation‑industry first.
- `●` **Spatial audio** — immersive 3D soundscape for alerts/comfort/safety; up to four connected speakers.
- `●` **Built‑in Global SIM / LTE** — cellular connectivity integrated into the system (two LTE antennas), underpinning cloud, live weather and tracking.
- `●` **LX Cloud** — "Aviation Command Center" for management/sync (cloud.lxnavigation.com), paired with a web Configurator.
- `●` **Navia Grip HMI** — dedicated hand controller: physical control, haptic warnings (e.g. approaching stall), heated grip, one‑touch voice‑assistant activation; two Grips for two‑seaters.
- `●` **High‑resolution displays** — 1920×1200, up to 323 ppi, 60 Hz refresh, up to 2500 nits, FALD backlight; sizes 4″ / 7″ / 12″; portrait or landscape; 80 mm standard mount; secondary/vario indicator options.
- `●` **ADS‑B in** — via the *Dual Flarm & ADS‑B* variant of Navia Traffic.
- `●` **AHRS / attitude** — Navia Sense (AHRS‑equipped variant) drives an artificial‑horizon / attitude widget; a non‑AHRS air‑data‑only variant is also offered.
- `●` **PoE power+data** — up to 100 W at 48 V and 1 Gbps over a single Ethernet cable; Navia Hub as the networking/power hub.
- `●` **Navia UPS** — integrated uninterruptible/backup power management.
- `◐` **Powered‑aircraft support** — Navia EMU (engine‑monitoring unit) and Navia MOP (means‑of‑propulsion) devices, high‑speed and AoA air‑data variants; targets powered/self‑launch/sustainer installations. Feature depth undocumented.
- `●` **Open hardware + documented protocols** — LX markets open‑source hardware and documented communication protocols to enable third‑party integration.
- `●` **Modular, multi‑display, multi‑seat architecture** — Core Pro + any number of displays, Hub, Sense, Traffic, UPS, Emu, Mop, Grip; dual‑cockpit configurations supported.

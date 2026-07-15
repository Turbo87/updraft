# Research: Condor flight recording format (`.ftr`)

Research notes on how [Condor — The Competition Soaring Simulator](https://www.condorsoaring.com/)
records flights, written to inform any future Condor-related work in Updraft
(the design docs already name a "Condor interface" as a TCP/UDP data source —
see [testing.md](../design/testing.md) and [devices.md](../design/devices.md)).

- **Date researched:** 2026-07-15
- **Condor versions in scope:** Condor 1.x, Condor 2, Condor 3
- **Confidence legend:** `[doc]` stated in vendor/primary docs · `[obs]` reported
  by community tool authors who have parsed the files · `[inf]` inferred from
  behaviour, not directly confirmed · `[?]` unknown / could not be verified

## TL;DR

- Condor stores each flight as a **proprietary binary `.ftr` file** ("flight
  track recording"). It is the sim's native flight record and the file Condor
  Club scores. `.ftr` is **not** documented publicly and **not** human-readable.
- Positions inside an `.ftr` are **landscape-relative Cartesian `X`/`Y` in
  metres** (derived from the landscape's UTM projection), **not** lat/lon.
  Converting to geographic coordinates requires the matching landscape's `.trn`
  terrain file plus Condor's `NaviCon.dll`.
- Each sample carries more than an IGC fix: besides position and altitude it
  includes **aircraft attitude** (heading/yaw, pitch, bank), sampled at a fixed
  high rate. Condor can export an `.ftr` to **IGC**, downsampling/interpolating
  to IGC's 1 Hz B-records.
- Do **not** confuse the `.ftr` *file* with Condor's **live UDP telemetry
  output** (NMEA + an extended `parameter=value` stream). That live feed — not
  the `.ftr` file — is what an app like Updraft would consume as a "Condor
  device". The two are documented separately below.

## Two different things people call a "Condor recording"

| | `.ftr` flight track file | Live UDP output |
|---|---|---|
| Nature | On-disk binary file, written after/while flying | Real-time network stream while flying |
| Contents | Full trajectory + attitude for replay/scoring | Per-frame telemetry (NMEA + extended fields) |
| Coordinates | Landscape `X`/`Y` metres | lat/lon (in the NMEA sentences) |
| Documented? | No (proprietary, reverse-engineered piecemeal) | Yes (config + field list are public) |
| Relevance to Updraft | Import/replay a past Condor flight | The "Condor interface" live data source |

The rest of this note treats them in that order.

---

## 1. The `.ftr` flight track file

### 1.1 What it is and where it lives

An `.ftr` is Condor's native flight record, functionally analogous to an IGC
file: it is what you load in Condor's **Flight Analysis** screen, what you
replay as a **"ghost"**, and what you upload to **Condor Club** for scoring.
Ghosts are literally `.ftr` files. `[doc]`

Storage location depends on the version/install, but the modern default is the
current Windows user's Documents tree:

- Condor 2: `…\Documents\Condor\FlightTracks\` `[doc]`
- Condor 3: `…\Documents\Condor3\FlightTracks\` `[doc]`
- Older/portable installs put a `FlightTracks\` folder under the Condor program
  directory (e.g. `C:\Program Files (x86)\Condor\FlightTracks`). `[doc]`

If you finish a flight without explicitly saving, Condor writes
**`LastTrack.ftr`** into that folder; it is overwritten on the next flight, so
it must be renamed to be kept. `[doc]`

### 1.2 Container and encoding

- The file is **binary and proprietary**. It does not open meaningfully in a
  text editor, and Condor ships no format specification. `[doc]`
- There is **no publicly documented byte layout**. The dedicated Condor-forum
  thread literally titled *"Reading FTR (Flight Track) file"* asking how to
  decode it went **unanswered**, and the main third-party converter (CoFliCo)
  is closed-source. Treat any byte-offset claim as unverified. `[obs]` `[?]`
- The one converter author who has built both directions describes an `.ftr`
  as holding the flight's **track/fixes plus orientation**, injected from or
  extracted to IGC B-records — consistent with a header + a fixed-rate array of
  sample records, but the exact framing is not published. `[obs]` `[inf]`

> Practical implication: there is currently no reliable way to parse `.ftr`
> directly from published specs. The supported path to usable data is
> **conversion to IGC** (§2), which is what every community tool does.

### 1.3 Coordinate system — the important part

Condor does **not** store lat/lon in the flight track. Positions are
**landscape-relative Cartesian coordinates**, `X` (easting) and `Y` (northing)
in **metres**, defined by that landscape's projection (built in the Condor
Landscape Editor from the **UTM** system, at 0.1 m editor accuracy). `[doc]`

Consequences:

- To turn a Condor position into lat/lon you need the **specific landscape**'s
  terrain file, `Landscapes\<name>\<name>.trn`, and Condor's coordinate
  library **`NaviCon.dll`**. `[doc]`
- `NaviCon.dll` exposes (confirmed via the `pycondor`/`Condor2Nav` bindings):
  - `NaviConInit(const char* trnPath)` — load a landscape's `.trn`
  - `GetMaxX()` / `GetMaxY()` — landscape extent in metres (valid `X`/`Y` range)
  - `XYToLon(float x, float y)` / `XYToLat(float x, float y)` — `X`/`Y` → degrees
    `[obs]`
- The projection round-trips at roughly **11 m** worst-case accuracy when going
  lon/lat → `X`/`Y` and back, per the landscape-editor documentation. `[doc]`
- This is why an `.ftr` is meaningless without knowing its landscape: the same
  `(X, Y)` maps to totally different places in different landscapes. The
  landscape name is recorded in the associated flight plan (`.fpl`, an
  INI-style text file) and echoed in Condor's own IGC export header. `[obs]`

### 1.4 What a sample contains

More than an IGC fix. Beyond position (`X`/`Y`) and altitude, the track records
**aircraft attitude** — heading/yaw, pitch, and bank — so Condor can replay the
ship banking into turns, not just translating along the ground. Evidence: the
IGC→FTR tool author notes that when attitude is *reconstructed* from a bare IGC
(which lacks it), the replayed aircraft "turns on the yaw axis only, and not
smoothly… in one move", i.e. the native format normally carries real attitude
data that a plain IGC cannot supply. `[obs]` `[inf]`

Sampling:

- Recorded at a **fixed, sub-second rate** (well above IGC's 1 Hz). The exact
  interval is not published; the fact that FTR→IGC converters must **downsample**
  and IGC→FTR converters must **interpolate/extrapolate** to fill the gaps
  confirms the native rate is higher and regular. `[obs]` `[inf]` `[?]`
- Because IGC fixes are sparser, CoFliCo offers several interpolation schemes
  when going the *other* way (IGC-like spacing → IGC): next / previous / nearest
  point-in-time, linear, and parabolic (most precise). `[doc]`

### 1.5 Validation / anti-cheat

Condor ships a **Track File Validator** ("Condor File Validator") that checks
whether an `.ftr` (or exported `.igc`) is **authentic and unaltered** — the
mechanism competition organisers use for offline events and that Condor Club
relies on for scoring. `[doc]`

- Implication: `.ftr` (and Condor's IGC export) carry an integrity/authenticity
  token — analogous in spirit to the IGC **G-record** security signature —
  that the validator recomputes. The exact algorithm is not public. `[doc]` `[inf]`
- Confirming this: the third-party CoFliCo converter explicitly **cannot produce
  validatable files** because "no checksums are computed" — i.e. only Condor
  itself can emit a track that passes its own validator. `[doc]`

### 1.6 Version differences (C2 vs C3)

- Flight **plans** (`.fpl`) are known to differ between Condor 2 and Condor 3
  (e.g. inversion-layer height is AGL in C2 but AMSL in C3) and need conversion
  (the `FPL2V3` tool). `[doc]`
- Whether the `.ftr` **track** binary layout changed between C2 and C3 is **not
  documented**; community converters historically target "Condor 2" explicitly,
  and C3 changed the default folder to `Condor3\`. Cross-version `.ftr`
  compatibility should be treated as unverified. `[obs]` `[?]`

---

## 2. `.ftr` → IGC conversion (the practical data path)

Since `.ftr` is opaque, all real workflows convert to IGC:

- **Condor itself** — Flight Analysis can export the loaded track to IGC. This
  is the only path that also produces a **validator-passing** IGC. `[doc]`
- **Condor Club** — serves both the original `.ftr` and a converted `.igc` per
  uploaded flight. `[doc]`
- **CoFliCo** (COndor FLIght track COnverter, condorutill.fr) — standalone/batch
  `.ftr` → `.igc`. Needs the landscape's `.trn` (for the `NaviCon.dll`
  `X`/`Y` → lat/lon step), supports the interpolation schemes above, and can
  also rename files by flight metadata (date, comp-ID, glider, landscape, start
  time). **Condor 2 only**, and its output is **not** validatable. `[doc]`
- **Downloader utilities** (e.g. `ryanwoodie/Condor.Club-FTR-IGC-Downloader`,
  CupX tools) — orchestrate bulk download from Condor Club and shell out to
  CoFliCo/7-Zip for the actual conversion; they do **not** parse `.ftr`
  themselves. `[obs]`

Condor's exported IGC header carries sim-specific context — landscape name,
glider type, Condor version, comp-ID — which is what lets downstream tools
label and group flights. `[obs]`

> Accuracy caveats worth remembering if Updraft ever ingests converted Condor
> IGCs: minor rounding differences between converters; interpolation choices
> change intermediate fixes; and third-party conversions lose the security
> signature. `[doc]`

---

## 3. Live UDP telemetry output (the "Condor interface")

Separate from the `.ftr` file, Condor can **stream telemetry over UDP** while
you fly. This is the feed relevant to Updraft's device layer, and unlike the
`.ftr` file it **is** documented and configurable.

### 3.1 Configuration

- Controlled by **`UDP.ini`** in the Condor install directory: `Enabled=1`,
  destination **host/IP** and **port**, and **`SendIntervalMs`** (packet
  period). `[doc]`
- Optional richer payloads via **`ExtendedData=1`** / **`ExtendedData1=1`**. `[doc]`
- Payload is **ASCII `parameter=value`** pairs; the NMEA subset makes it look
  like a normal GPS/vario source to navigation software. `[doc]`

### 3.2 NMEA subset

Condor emits standard sentences so existing soaring apps can treat it as a
connected instrument:

- `GPGGA` (position/altitude), `GPRMC` (position/speed/track), and **`LXWP0`**
  (LX-style vario/altitude/speed). The set "can be expanded if needed." `[doc]`
- Updraft already parses `LXWP0` (`libs/updraft_nmea/src/sentences/lx/lxwp0.rs`)
  and standard GPS sentences, so the NMEA half of a Condor feed is largely
  covered by the existing parser.

### 3.3 Extended field set

With extended data enabled, the stream adds a rich per-frame model (units as
reported by community consumers such as `docop/Condor2Arduino`): `[obs]`

- **Time:** `time` (in-sim time, decimal hours)
- **Altitude / vario:** `altitude` (m), `vario`, `evario` (electronic),
  `nettovario`, `integrator` (all m/s)
- **Attitude:** `compass` (deg); `yaw`, `pitch`, `bank` (rad);
  `quaternionx/y/z`
- **Rates:** `turnrate`, `rollrate`, `pitchrate`, `yawrate` (rad/s)
- **Kinematics:** `ax/ay/az` (m/s²), `vx/vy/vz` (m/s), `gforce`
- **Air/attitude cues:** `slipball` (rad), `yawstringangle` (rad),
  `turbulencestrength`, `surfaceroughness`
- **Geometry:** `height` (CG above ground, m), `wheelheight` (m)
- **Aircraft state / controls:** `flaps` (index), `MC` (m/s), `water` (kg),
  `radiofrequency` (MHz), `hudmessages` (semicolon-separated text)

Note the overlap with §1.4: the same attitude/kinematic quantities Condor
streams live are the kind of state the `.ftr` file must store to reproduce a
flight — so the live field list is a good *model* of the recorded state, even
though the on-disk encoding is different and undocumented.

---

## 4. Implications for Updraft

- **Live "Condor device":** consume the UDP feed. The NMEA subset (`GPGGA`,
  `GPRMC`, `LXWP0`) is already within the `updraft_nmea` parser's scope; a
  Condor source is mostly a UDP transport + enabling `LXWP0`. The extended
  `parameter=value` stream is optional and would need its own small parser only
  if we want vario/attitude beyond NMEA.
- **Importing a past Condor flight:** target **IGC**, not `.ftr`. There is no
  published `.ftr` spec to implement against, and only Condor emits
  validator-passing output. Converted Condor IGCs flow through Updraft's normal
  IGC path.
- **If direct `.ftr` parsing is ever required:** it is a reverse-engineering
  project (no spec, no open reference implementation), and it would still need
  the per-landscape `.trn` + a re-implementation of `NaviCon.dll`'s `X`/`Y` →
  lat/lon projection to produce geographic fixes. Recommend against unless there
  is a strong reason IGC export cannot be used.

## 5. Open questions / not verified

- Exact `.ftr` byte layout: header/magic, version field, record size, sample
  interval, field order. `[?]`
- Whether the sample rate is constant across gliders/versions. `[?]`
- The validation/signature algorithm and whether it differs between `.ftr` and
  Condor's IGC export. `[?]`
- Whether C2 and C3 `.ftr` files are binary-compatible. `[?]`
- The precise projection `NaviCon.dll` implements per landscape (UTM zone
  handling, datum). `[?]`

## Sources

Vendor / primary:
- [Condor — official site](https://www.condorsoaring.com/) and
  [Condor 2 manual (PDF)](https://www.condorsoaring.com/wp-content/uploads/2021/09/condor-2-manual-1.pdf)
- [Condor Help: Submitting a Flight Track to Condor Club](https://condor-help.helpscoutdocs.com/article/48-submitting-a-flight-track-to-condor-club)
- Condor forum: [where the FlightTracks folder is](https://www.condorsoaring.com/forums/viewtopic.php?t=18854),
  [LastTrack.ftr](https://www.condorsoaring.com/forums/viewtopic.php?t=20875),
  ["Reading FTR (Flight Track) file" (unanswered decode request)](https://www.condorsoaring.com/forums/viewtopic.php?t=20924),
  ["Reading FTR files"](https://www.condorsoaring.com/forums/viewtopic.php?t=22510),
  [IGC→FTR replay tool (attitude/interpolation notes)](https://www.condorsoaring.com/forums/viewtopic.php?t=7218),
  [lon/lat ↔ Condor XY](https://www.condorsoaring.com/forums/viewtopic.php?t=12474)

Format / usage descriptions:
- [file-extensions.org — FTR (Condor flight track)](https://www.file-extensions.org/ftr-file-extension-condor-flight-track)
- [Soaring Club of Houston — Flight Track Files](https://sites.google.com/site/soaringclubofhouston/learn-to-fly/getting-started/condor-flight-simulator/condor-files/flight-track-files)
- [Condor Track File Validator (Filefacts)](http://www.filefacts.com/condor-track-file-validator-info)

Conversion & coordinate tooling:
- [CoFliCo README (FTR→IGC, .trn dependency, interpolation, "no checksums")](http://condorutill.fr/CoFliCo/CoFliCo_README.txt)
  and [condorutill.fr tools](http://www.condorutill.fr/)
- [mpusz/Condor2Nav — `condor.cpp` (NaviCon.dll usage, X/Y metres, .trn)](https://github.com/mpusz/Condor2Nav/blob/master/src/condor.cpp)
- [scls19fr/pycondor — `condor_dll.py` (NaviCon.dll bindings)](https://github.com/scls19fr/pycondor/blob/master/pycondor/condor_dll.py)
- [ryanwoodie/Condor.Club-FTR-IGC-Downloader](https://github.com/ryanwoodie/Condor.Club-FTR-IGC-Downloader)

Live UDP/NMEA output:
- [docop/Condor2Arduino — UDP field list](https://github.com/docop/Condor2Arduino)
- [XCSoar issue #2488 — Condor UDP.ini vs NMEA driver](https://github.com/XCSoar/XCSoar/issues/2488)

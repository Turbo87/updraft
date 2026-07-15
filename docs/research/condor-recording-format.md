# Research: Condor flight recording format (`.ftr`)

Research notes on how [Condor — The Competition Soaring Simulator](https://www.condorsoaring.com/)
records flights, written to inform any future Condor-related work in Updraft
(the design docs already name a "Condor interface" as a TCP/UDP data source —
see [testing.md](../design/testing.md) and [devices.md](../design/devices.md)).

- **Date researched:** 2026-07-15
- **Primary evidence:** one real Condor `.ftr` file was reverse-engineered
  byte-for-byte (Slovenia3 landscape, JS3 glider, 87 639 samples, 5.63 MB). Its
  layout is documented in §2. Everything in §2 is derived from that single file
  and cross-checked against the community facts in §1/§3.
- **Condor versions in scope:** Condor 1.x, Condor 2, Condor 3
- **Confidence legend:** `[bin]` established directly from the sample file ·
  `[doc]` stated in vendor/primary docs · `[obs]` reported by community tool
  authors who have parsed the files · `[inf]` inferred from behaviour, not
  directly confirmed · `[?]` unknown / could not be verified

## TL;DR

- Condor stores each flight as a **binary `.ftr` file** ("flight track
  recording") — the sim's native record and the file Condor Club scores. There
  is no vendor spec, but the format is straightforward once decoded.
- The file is: an ASCII **`FTR`** magic + a header (pilot, aircraft, landscape,
  task), a **`uint32` sample count**, a flat array of **fixed 64-byte sample
  records**, a small summary footer, and a trailing **64-character validation
  signature**. `[bin]`
- Each 64-byte sample holds `float32` **time, position (X, Y, Z), a unit
  attitude quaternion, and a cumulative distance**, plus a 24-byte trailer that
  does not decode as floats (likely packed/obfuscated validation data). `[bin]`
- Positions are **landscape-relative Cartesian `X`/`Y`/`Z` in metres** (from the
  landscape's UTM projection), **not** lat/lon. Converting to geographic
  coordinates needs the matching landscape's `.trn` file plus Condor's
  `NaviCon.dll`. `[bin]` `[doc]`
- Sampling is dense — **~11–12 Hz** in the sample (mean ≈ 0.086 s between
  samples) — which is why FTR→IGC converters downsample to IGC's 1 Hz. `[bin]`
- Do **not** confuse the `.ftr` *file* with Condor's **live UDP telemetry
  output** (NMEA + an extended `parameter=value` stream). That live feed — not
  the `.ftr` file — is what an app like Updraft would consume as a "Condor
  device". Covered in §4.

## Two different things people call a "Condor recording"

| | `.ftr` flight track file | Live UDP output |
|---|---|---|
| Nature | On-disk binary file, written after/while flying | Real-time network stream while flying |
| Contents | Full trajectory + attitude for replay/scoring | Per-frame telemetry (NMEA + extended fields) |
| Coordinates | Landscape `X`/`Y` metres | lat/lon (in the NMEA sentences) |
| Documented? | No vendor spec (decoded in §2) | Yes (config + field list are public) |
| Relevance to Updraft | Import/replay a past Condor flight | The "Condor interface" live data source |

---

## 1. The `.ftr` flight track file (background)

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

### 1.2 Coordinate system

Condor does **not** store lat/lon in the flight track. Positions are
**landscape-relative Cartesian coordinates**, `X` (easting) and `Y` (northing)
in **metres**, defined by that landscape's projection (built in the Condor
Landscape Editor from the **UTM** system, at 0.1 m editor accuracy). `[doc]`
The sample file confirms this directly (§2). Consequences:

- To turn a Condor position into lat/lon you need the **specific landscape**'s
  terrain file, `Landscapes\<name>\<name>.trn`, and Condor's coordinate
  library **`NaviCon.dll`**. `[doc]`
- `NaviCon.dll` exposes (confirmed via the `pycondor`/`Condor2Nav` bindings):
  `NaviConInit(const char* trnPath)`, `GetMaxX()` / `GetMaxY()` (landscape
  extent, metres), and `XYToLon(x, y)` / `XYToLat(x, y)`. `[obs]`
- The projection round-trips at roughly **11 m** worst-case accuracy. `[doc]`
- The same `(X, Y)` maps to different places in different landscapes, so an
  `.ftr` is meaningless without its landscape name — which the header records
  (`Slovenia3` in the sample). `[bin]`

### 1.3 Validation / anti-cheat

Condor ships a **Track File Validator** ("Condor File Validator") that checks
whether an `.ftr` (or exported `.igc`) is **authentic and unaltered** — the
mechanism competition organisers use for offline events and that Condor Club
relies on for scoring. `[doc]` The sample file ends with a **64-character
uppercase alphanumeric signature** (§2.6) that is almost certainly what the
validator recomputes — analogous in spirit to the IGC **G-record**. `[bin]` `[inf]`
The third-party CoFliCo converter explicitly **cannot produce validatable
files** ("no checksums are computed"), i.e. only Condor itself can emit a track
that passes its own validator. `[doc]`

### 1.4 Version differences (C2 vs C3)

Flight **plans** (`.fpl`) differ between Condor 2 and Condor 3 (e.g.
inversion-layer height is AGL in C2 but AMSL in C3) and need conversion (the
`FPL2V3` tool). `[doc]` Whether the `.ftr` **track** binary layout changed
between C2 and C3 is **not documented**; community converters historically
target "Condor 2" explicitly. The sample here is one version only, so treat
cross-version details as unverified. `[obs]` `[?]`

---

## 2. `.ftr` binary structure — reverse-engineered from a real file

All of §2 comes from decoding one real `.ftr` (little-endian throughout;
IEEE-754 `float32` unless noted). It is a single sample, so field *labels* carry
the confidence tags, but the *layout* (magic, count, 64-byte stride, footer,
signature) is unambiguous and closes to the exact file size.

### 2.1 File map

| Region | Offset | Size | Contents |
|---|---|---|---|
| Magic | `0x00` | 3 | ASCII `FTR` |
| Format byte | `0x03` | 1 | `0x1C` (= 28) — version/format tag `[inf]` |
| Header | `0x04` | … | version/flags, pilot, aircraft, landscape, task (§2.2–2.3) |
| Sample count | `0x54AB` (21675) | 4 | `uint32` = **87 639** (number of records) `[bin]` |
| — | 21679 | 4 | one `float32` (`0.0136`) before the array — role unclear `[?]` |
| **Track array** | **21683** | **87639 × 64** | flat array of 64-byte sample records (§2.4) |
| Footer | 5 630 579 | 88 | per-turnpoint summary + final distances (§2.5) |
| Signature | 5 630 667 | 64 | ASCII `[A-Z0-9]{64}` validation hash (§2.6) |
| EOF | 5 630 731 | | (header + 87639·64 + 88 + 64 = file size, exactly) |

The header's first bytes: `46 54 52 1c 0c 00 00 00 …` — `FTR`, `0x1C`, then a
`uint32` = 12 and several `float32`s (a recurring `13.0233` appears here and in
the footer — possibly a build/version constant). Their exact meaning is not
resolved. `[bin]` `[?]`

### 2.2 Header metadata (strings)

Text fields are **length-prefixed** (a 1-byte length, then that many ASCII
bytes) embedded among binary fields. From the sample:

| Field | Value | Meaning `[inf]` |
|---|---|---|
| pilot first name | `Philip` | |
| pilot last name | `S` | |
| country | `Germany` | |
| aircraft id / reg | `D-2611` | glider registration / CN |
| comp sign | `YB` | competition ID |
| landscape | `Slovenia3` | scenery — needed for `X/Y`→lat/lon |
| glider | `JS3-15` (`JS3-15s18S`) | plane type / class |
| livery | `KW.dds`, `BALLAAAARN` | texture file / paint text |

### 2.3 Task turnpoints

After the pilot/landscape block the **task** is stored as a list of fixed
**84-byte** turnpoint records, each: a length-prefixed name, then two positions
as `float32` **(X, Y, elevation)** in metres, then observation-zone parameters
(radius/angles). In the sample the task is `Lienz-Nikolsdorf` (listed twice —
takeoff + start) → `Paterzipf` → `Mauterndorf` → `Lesce-Bled`. The first
turnpoint decodes cleanly as `X=307036.6, Y=162198.6, elev=638.0`. `[bin]`

### 2.4 The 64-byte sample record (the core)

Each of the 87 639 records is 64 bytes: **ten `float32` fields (40 bytes)
followed by a 24-byte trailer**. Field-by-field, with observed ranges over the
whole flight:

| Off | Type | Field | Observed | Notes |
|---|---|---|---|---|
| +0 | f32 | **time** | 2.63191 → 2.71931 | monotonic sample clock; see §2.4.1 `[bin]` |
| +4 | f32 | **X** (easting, m) | 205 200 – 308 700 | landscape metres `[bin]` |
| +8 | f32 | **Y** (northing, m) | 110 600 – 198 000 | landscape metres `[bin]` |
| +12 | f32 | **Z** (altitude, m) | 506 – 2 771 | MSL metres `[bin]` |
| +16 | f32 | small signed | −0.012 – 0.024 | ~±0.02 (rad?); weak corr. with turn rate — **unresolved** `[?]` |
| +20 | f32 | **q0** (quat x) | ±0.9 | attitude quaternion `[bin]` |
| +24 | f32 | **q1** (quat y) | ±0.9 | |
| +28 | f32 | **q2** (quat z) | ±0.9 | |
| +32 | f32 | **q3** (quat w) | ±1.0 | scalar-last; \|q\|≈1 (§2.4.2) `[bin]` |
| +36 | f32 | **distance** (m) | 0 → 184 557 | cumulative, monotonic (§2.4.3) `[bin]` |
| +40 | 24 B | trailer | high-entropy | does **not** decode as floats — packed/obfuscated `[?]` |

Sanity checks that anchor the labels:

- **Record 0 sits on the runway at the departure airfield.** Its
  `X,Y,Z = 306928, 162121, 639.4` matches the `Lienz-Nikolsdorf` turnpoint
  (`307037, 162199, 638`) to ~110 m / ~1 m, and `distance = 0`. Record 1 is
  identical in position (glider stationary at start). `[bin]`
- **Altitude range 506–2771 m** is right for an alpine soaring flight, and the
  **last** sample is ~100 km away near the final turnpoint (a point-to-point
  task, not a return). `[bin]`

First record, annotated (bytes at offset 21683):

```
22 71 28 40   time = 2.63191
f6 dd 95 48   X    = 306927.69
3e 52 1e 48   Y    = 162120.97
b5 db 1f 44   Z    = 639.43
38 7c e9 ba   f4   = -0.00178
b9 b7 e5 bc   q0   = -0.02804
4a 06 1f 3d   q1   =  0.03882
6e db 9c be   q2   = -0.30636
b7 61 73 3f   q3   =  0.95071
00 00 00 00   dist =  0.0
12 52 52 05 7a fe a5 1f 6d 8a ae 8c   ┐ 24-byte trailer
5f a2 97 8b 0a 7a bd 52 64 1a c5 e9   ┘ (not float-decodable)
```

#### 2.4.1 The time field and sample rate

`+0` is monotonic and near-perfectly linear with the record index
(corr ≈ 0.99999) — a fixed-timestep sample clock. Interpreting it as **days**
(the only unit that yields a sane rate) gives a **2.10-hour** flight sampled at
a mean **~11.6 Hz** (mean Δt ≈ 0.086 s), with a handful of larger gaps (sim
pauses/stutters). It is stored as `float32`, so its resolution is only ~0.05 s.
The exact unit/epoch is not resolved (the raw value 2.63 is not a meaningful
absolute calendar date), but the *rate* is solid. `[bin]` `[inf]`

#### 2.4.2 The attitude quaternion

`+20..+32` are a **unit quaternion** (scalar-last): the sum of squares is 1.000
at the start and drifts down only to ~0.995 by the end (stored, not
renormalised each frame). This is the per-sample **aircraft attitude** — the
thing a plain IGC lacks, and why Condor can replay the ship banking rather than
just sliding along its ground track. `[bin]`

#### 2.4.3 The distance field

`+36` starts at exactly 0 and increases monotonically to **184 557 m**. Over the
2.10 h flight that is an **88 km/h average** — a textbook soaring cross-country
average — so it reads as a **cumulative odometer / achieved distance** (the kind
of value used for scoring), not raw straight-line displacement. `[bin]` `[inf]`

### 2.5 Footer (88 bytes)

Between the last record and the signature is a small summary block: a series of
`(uint32 index, float32 value, …)` entries — turnpoint indices with associated
values (times/distances), and near the end the **final distances**
(`184 556.9` — matching the last record's distance field — and `165 282.9`).
Interpretable as the flight's per-turnpoint and total-distance summary. `[bin]` `[inf]`

### 2.6 Validation signature (64 bytes)

The final 64 bytes are ASCII uppercase `[A-Z0-9]` — in the sample:

```
FR6IIRE997GZ9WX0PJJYVNVQGC3I75JJ919DUCQ0AJKSK38WY3XWHFNA0KXAGU0P
```

This is the file's **authenticity token**, recomputed by Condor's Track File
Validator (§1.3). The signing/hashing algorithm is not public, and altering any
earlier byte would invalidate it — which is exactly why third-party converters
cannot emit validatable tracks. `[bin]` `[inf]` `[?]`

> **Takeaway for tooling:** reading position/altitude/attitude/time out of an
> `.ftr` is entirely feasible from §2.4 alone; the only genuinely opaque parts
> are the 24-byte per-sample trailer and the trailing signature. Producing lat/
> lon still requires the landscape's `.trn` + `NaviCon.dll` projection.

---

## 3. `.ftr` → IGC conversion (the usual data path)

Because reading `.ftr` needs the landscape projection (and because only Condor
can emit validatable output), most workflows convert to IGC:

- **Condor itself** — Flight Analysis exports the loaded track to IGC. The only
  path that also produces a **validator-passing** IGC. `[doc]`
- **Condor Club** — serves both the original `.ftr` and a converted `.igc` per
  uploaded flight. `[doc]`
- **CoFliCo** (condorutill.fr) — standalone/batch `.ftr` → `.igc`. Needs the
  landscape's `.trn` (for the `NaviCon.dll` `X/Y`→lat/lon step), offers several
  interpolation schemes to resample the dense native rate down to IGC's 1 Hz,
  and can rename files by flight metadata. **Condor 2 only**; its output is
  **not** validatable. `[doc]`
- **Downloader utilities** (e.g. `ryanwoodie/Condor.Club-FTR-IGC-Downloader`,
  CupX tools) — orchestrate bulk Condor Club downloads and shell out to
  CoFliCo/7-Zip; they do **not** parse `.ftr` themselves. `[obs]`

Condor's exported IGC header carries sim-specific context (landscape name,
glider type, Condor version, comp-ID) — the same metadata seen in the `.ftr`
header (§2.2). `[obs]`

---

## 4. Live UDP telemetry output (the "Condor interface")

Separate from the `.ftr` file, Condor can **stream telemetry over UDP** while
you fly. This is the feed relevant to Updraft's device layer, and unlike the
`.ftr` file it **is** documented and configurable.

### 4.1 Configuration

- Controlled by **`UDP.ini`** in the Condor install directory: `Enabled=1`,
  destination **host/IP** and **port**, and **`SendIntervalMs`** (packet
  period). Optional richer payloads via **`ExtendedData=1`** /
  **`ExtendedData1=1`**. Payload is **ASCII `parameter=value`** pairs. `[doc]`

### 4.2 NMEA subset

Condor emits `GPGGA`, `GPRMC`, and **`LXWP0`** so existing soaring apps can
treat it as a connected instrument. Updraft already parses `LXWP0`
(`libs/updraft_nmea/src/sentences/lx/lxwp0.rs`) and standard GPS sentences, so
the NMEA half of a Condor feed is largely covered by the existing parser. `[doc]`

### 4.3 Extended field set

With extended data enabled, the stream adds a rich per-frame model (units as
reported by consumers such as `docop/Condor2Arduino`): `[obs]`

- **Time:** `time` (in-sim, decimal hours)
- **Altitude / vario:** `altitude` (m); `vario`, `evario`, `nettovario`,
  `integrator` (m/s)
- **Attitude:** `compass` (deg); `yaw`, `pitch`, `bank` (rad); `quaternionx/y/z`
- **Rates:** `turnrate`, `rollrate`, `pitchrate`, `yawrate` (rad/s)
- **Kinematics:** `ax/ay/az` (m/s²), `vx/vy/vz` (m/s), `gforce`
- **Cues/geometry:** `slipball`, `yawstringangle` (rad), `turbulencestrength`,
  `surfaceroughness`, `height` (CG above ground, m), `wheelheight` (m)
- **State/controls:** `flaps` (index), `MC` (m/s), `water` (kg),
  `radiofrequency` (MHz), `hudmessages` (text)

The overlap with §2.4 is notable: the live stream carries the same
attitude/kinematic quantities the `.ftr` must store to reproduce a flight (the
UDP stream even exposes a `quaternion`, echoing the recorded attitude
quaternion). The live field list is thus a good *model* of the recorded state,
even though the on-disk encoding differs.

---

## 5. Implications for Updraft

- **Live "Condor device":** consume the UDP feed. The NMEA subset (`GPGGA`,
  `GPRMC`, `LXWP0`) is already within the `updraft_nmea` parser's scope; a
  Condor source is mostly a UDP transport + enabling `LXWP0`. The extended
  `parameter=value` stream is optional and would need its own small parser only
  if we want vario/attitude beyond NMEA.
- **Importing a past Condor flight:** IGC is the low-effort path (Condor/Condor
  Club/CoFliCo already produce it, and it flows through Updraft's normal IGC
  pipeline). Direct `.ftr` parsing is now demonstrably feasible (§2.4) if we
  ever want attitude or the native sample rate — but it still needs the
  per-landscape `.trn` + a re-implementation of `NaviCon.dll`'s projection to
  produce geographic fixes, and it cannot verify or reproduce the validation
  signature.
- **If we ever add an `.ftr` reader** (e.g. as an `updraft_*` parser or a test
  fixture path): the structure in §2 is enough to extract time, X/Y/Z, and
  attitude. Keep the landscape name from the header to pick the right `.trn`.

## 6. Open questions / not verified

- The 24-byte per-sample **trailer** (record `+40..+63`): does not decode as
  floats; likely packed flags/obfuscated or validation-related. `[?]`
- The small **`+16` field** (~±0.02): sideslip/yaw-string angle, a rate, or
  something else — weakly correlates with turn rate only. `[?]`
- Exact **unit/epoch of the time field** (rate is solid at ~11–12 Hz; the
  absolute meaning of the `float32` value is not). `[?]`
- Meaning of the header `float32`s (the recurring `13.0233`, the `uint32=12`)
  and the pre-array `float32` at offset 21679. `[?]`
- The **validation signature algorithm**, and whether it differs between `.ftr`
  and Condor's IGC export. `[?]`
- Whether **C2 and C3** `.ftr` files are binary-compatible, and whether the
  record layout is stable across gliders/versions (only one sample examined). `[?]`
- The precise projection `NaviCon.dll` implements per landscape (UTM zone,
  datum). `[?]`

## Sources

Reverse-engineered directly from one real Condor `.ftr` sample (Slovenia3 /
JS3, 87 639 samples). Community/vendor context:

Vendor / primary:
- [Condor — official site](https://www.condorsoaring.com/) and
  [Condor 2 manual (PDF)](https://www.condorsoaring.com/wp-content/uploads/2021/09/condor-2-manual-1.pdf)
- [Condor Help: Submitting a Flight Track to Condor Club](https://condor-help.helpscoutdocs.com/article/48-submitting-a-flight-track-to-condor-club)
- Condor forum: [FlightTracks folder location](https://www.condorsoaring.com/forums/viewtopic.php?t=18854),
  [LastTrack.ftr](https://www.condorsoaring.com/forums/viewtopic.php?t=20875),
  ["Reading FTR (Flight Track) file" (unanswered decode request)](https://www.condorsoaring.com/forums/viewtopic.php?t=20924),
  [IGC→FTR replay tool (attitude/interpolation notes)](https://www.condorsoaring.com/forums/viewtopic.php?t=7218),
  [lon/lat ↔ Condor XY](https://www.condorsoaring.com/forums/viewtopic.php?t=12474)

Format / usage descriptions:
- [file-extensions.org — FTR (Condor flight track)](https://www.file-extensions.org/ftr-file-extension-condor-flight-track)
- [Soaring Club of Houston — Flight Track Files](https://sites.google.com/site/soaringclubofhouston/learn-to-fly/getting-started/condor-flight-simulator/condor-files/flight-track-files)
- [Condor Track File Validator (Filefacts)](http://www.filefacts.com/condor-track-file-validator-info)

Conversion & coordinate tooling:
- [CoFliCo README (FTR→IGC, .trn dependency, interpolation, "no checksums")](http://condorutill.fr/CoFliCo/CoFliCo_README.txt)
- [mpusz/Condor2Nav — `condor.cpp` (NaviCon.dll usage, X/Y metres, .trn)](https://github.com/mpusz/Condor2Nav/blob/master/src/condor.cpp)
- [scls19fr/pycondor — `condor_dll.py` (NaviCon.dll bindings)](https://github.com/scls19fr/pycondor/blob/master/pycondor/condor_dll.py)
- [ryanwoodie/Condor.Club-FTR-IGC-Downloader](https://github.com/ryanwoodie/Condor.Club-FTR-IGC-Downloader)

Live UDP/NMEA output:
- [docop/Condor2Arduino — UDP field list](https://github.com/docop/Condor2Arduino)
- [XCSoar issue #2488 — Condor UDP.ini vs NMEA driver](https://github.com/XCSoar/XCSoar/issues/2488)

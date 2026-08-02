# IGC Replay Through NMEA

## Purpose

The `updraft_replay` development tool replays recorded NMEA bytes through a
TCP server. Updraft also needs this tool to replay IGC flight files through the
same TCP and NMEA ingestion path.

IGC replay converts recorded IGC values to NMEA sentences. It does not derive
values that the file does not contain. The generated sentences prepare the
replay tool for future support of pressure altitude and LXNAV inputs in the
core.

This specification extends
[NMEA Replay Server](2026-07-31-nmea-replay-design.md). It replaces that
specification's exclusion of IGC parsing, NMEA generation, and normalization.
All other NMEA replay, TCP server, playback, skip, loop, progress, and client
behavior stays unchanged.

## Scope

This change provides these items:

- IGC parsing with the `igc` crate from crates.io.
- Conversion of selected IGC records and extensions to typed NMEA messages.
- NMEA wire encoding for the six sentence types that IGC replay generates.
- Real-time scheduling from IGC B and K record times.
- Best-effort handling of malformed IGC records and extension values.
- Support for IGC recordings that cross midnight.
- Strict `.nmea` and `.igc` input selection.

This change does not provide these items:

- An in-app replay mode or user interface.
- Playback-speed controls.
- Artificial TCP write boundaries.
- New NMEA consumers in `updraft_core`.
- Values derived from consecutive fixes or other inferred state.
- IGC task, signature, satellite-list, event, or opaque manufacturer data.
- Conversion of ACZ, FXA, ENL, NET, AOR, AOP, or AOA extensions.
- Conversion of LXNAV `L` records.

## Input selection

The command accepts only these filename extensions:

- `.nmea` selects byte-preserving NMEA replay.
- `.igc` selects IGC parsing and NMEA generation.

Extension matching is case-insensitive. A missing extension, a non-UTF-8
extension, or any other extension is an error. The tool reports the error
before it reads or parses the file.

The command interface stays:

```text
updraft_replay [--listen <ADDRESS>] [--loop] [--skip <SECONDS>] <FILE>
```

## Shared replay model

`Replay` remains the shared ordered collection of timed `ReplayEvent` values.
The server and playback clock do not know which source format produced an
event.

The replay package provides two constructors:

- `Replay::from_nmea()` keeps the current byte-preserving NMEA behavior.
- `Replay::from_igc()` parses IGC records and owns generated NMEA bytes.

Generated sentences with the same timestamp form one replay event. The event
payload contains each complete sentence in its specified order. Each sentence
has its own checksum and line ending.

The first usable B or K timestamp becomes replay time zero. A file is not
replayable unless it contains at least one usable B record.

The existing `--skip` and `--loop` behavior applies to IGC events. The server
still starts its clock after it binds the listener. Playback still continues
without clients. A late client still receives only future events.

## NMEA wire encoding

`updraft_nmea` owns NMEA wire encoding. IGC replay does not build sentence
strings itself.

The crate implements these conversions:

```rust
impl TryFrom<&Rmc> for Vec<u8>
impl TryFrom<&Gga> for Vec<u8>
impl TryFrom<&Pgrmz> for Vec<u8>
impl TryFrom<&Lxwp0> for Vec<u8>
impl TryFrom<&Lxwp1> for Vec<u8>
impl TryFrom<&Plxvs> for Vec<u8>
```

No conversion is added for `Message` or another sentence type. Unsupported
sentence types therefore fail at compile time instead of returning a runtime
unsupported-message error.

Each conversion writes a complete sentence. It includes the `$` start marker,
comma-separated fields, an uppercase XOR checksum, and `\r\n`.

An absent optional value becomes an empty field. The encoder rejects these
values:

- An invalid custom talker code.
- A date with a year outside `2000..=2099`.
- A free-form text field that contains a comma, `*`, carriage return, line
  feed, or a non-ASCII character.

Time uses `HHMMSS.sss`. Date uses `DDMMYY`. Latitude uses `DDMM.mmmmm` and
longitude uses `DDDMM.mmmmm`, with separate hemisphere fields. Other decimal
fields use the default `f64` representation. The encoder does not repeat
domain validation for floating-point values or coordinates.

## IGC parsing

The IGC loader processes one input line at a time. It uses
`igc::records::Record::parse_line()` and the crate's typed records. It keeps
the valid A, H, I, and J information that later records need.

The loader uses these record types:

- A supplies the recorder unique ID.
- H supplies the date, recorder product, software version, and hardware
  version.
- I defines extensions on B records.
- J defines extensions on K records.
- B supplies fixes, altitudes, and the main recorded extensions.
- K supplies wind extensions.

Other record types do not produce replay data. The loader ignores them without
warnings.

The loader reads extension values only when a valid I or J definition declares
the matching mnemonic. A missing definition or extension is an absent value.
The loader does not use fixed fixture-specific byte positions.

## Date and time

The `DTE` H record initializes the calendar date. The loader uses the first
`DDMMYY` value before an optional comma suffix.

RMC includes the current date when this header is valid. RMC still contains
time, status, position, speed, and track when the date is missing or malformed.
Its date field is empty in that case.

A backward time change of more than 12 hours means that the recording crossed
midnight. The scheduler adds one day. It also increments the RMC date with
normal Gregorian month, year, and leap-year behavior.

A smaller backward change does not move the replay clock backward. The
affected record uses the current replay time. The loader reports a timestamp
regression warning.

## B record conversion

Every usable B record generates applicable sentences in this order:

1. `$GPRMC`
2. `$GPGGA`
3. `$PGRMZ`, when pressure altitude is present
4. `$LXWP0`, when TAS or VAT is present
5. `$PLXVS`, when OAT is present

Generated RMC and GGA sentences use the `GP` talker code.

### RMC

RMC maps these values:

- B timestamp to UTC time.
- B fix validity to active or void status.
- B latitude and longitude to position.
- GSP to speed over ground after conversion from hundredths of km/h to knots.
- TRT to true course in degrees.
- The tracked header date to the optional date field.

GSP and TRT remain optional. Their RMC fields stay empty when the I record does
not define them, when the B record does not contain them, or when their values
are malformed.

### GGA

GGA maps these values:

- B timestamp to UTC time.
- B latitude and longitude to position.
- A valid B fix to GPS fix quality.
- A navigation warning to invalid fix quality.
- SIU to the optional satellite count.
- B GNSS altitude to MSL altitude and geoid separation.

The B GNSS altitude is height above the WGS84 ellipsoid. `updraft_egm96`
provides the geoid undulation at the recorded position. GGA altitude is the
ellipsoid height minus the undulation. GGA geoid separation is the
undulation.

A zero GNSS altitude means that altitude is missing. GGA leaves both altitude
and geoid separation empty. It still contains the other available fields.

### PGRMZ

IGC pressure altitude is read in meters and stored as a `Length`. The PGRMZ
encoder writes the value in feet with the default `f` unit field. A valid B
fix uses the three-dimensional fix indicator. A navigation warning uses the
no-fix indicator.

A zero pressure altitude means that altitude is missing. PGRMZ is not emitted
for that B record. The LXWP0 and PLXVS pressure-altitude fields stay empty.

### LXWP0 from B

LXWP0 maps these optional values:

- B pressure altitude to pressure altitude.
- TAS to true airspeed after conversion from hundredths of km/h.
- VAT to the first vario sample after conversion from hundredths of m/s.

The logger and heading fields stay empty. The wind fields stay empty unless a
K record at the same timestamp supplies wind values.

### PLXVS

PLXVS maps OAT to outside air temperature after conversion from tenths of a
degree Celsius. It also maps B pressure altitude to the recorder pressure
altitude field. Its other fields stay empty.

## K record conversion

K records map these optional extensions:

- WDI to wind direction in degrees.
- WSP to wind speed after conversion from hundredths of km/h.

When a B and K record have the same absolute timestamp, their fields form one
LXWP0 sentence at that timestamp. A K record without a matching B record
generates a wind-only LXWP0 sentence.

The loader does not carry the last wind value into a later timestamp. It does
not infer wind when WDI or WSP is absent.

## Recorder identification

LXWP1 maps these values:

- Product uses the final comma-separated component of the `FTY` H record.
- Serial uses the A-record unique ID.
- Software version uses the `RFW` H record.
- Hardware version uses the `RHW` H record.
- License stays empty.

A missing or malformed source leaves only its related field empty. The loader
still generates LXWP1 when all identification fields are empty.

The first LXWP1 event occurs at replay time zero. Another event occurs every
60 seconds through the replay duration. A loop restarts this schedule at time
zero.

When an LXWP1 event shares a timestamp with flight data, LXWP1 is the first
sentence in the event payload.

## Best-effort errors

IGC parsing uses best-effort behavior. The loader collects warnings. The
command logs them after schedule construction.

Each warning identifies the input line and the affected record or field.
These conditions produce warnings and continue replay:

- A malformed A, H, I, J, B, or K record that affects mapped data.
- A malformed mapped extension value.
- A mapped value that cannot form a typed NMEA field.
- A generated sentence that fails NMEA encoding.
- A smaller timestamp regression.

A malformed B record is skipped. A malformed K record does not affect its
matching B record. A malformed extension omits only its related NMEA field or
sentence.

These conditions stop startup:

- The file has no usable B record.
- The generated schedule has no timed event.
- `--skip` exceeds the replay duration.
- A normal file, listener, or command-line error occurs.

## Automated tests

NMEA encoding follows red-green-refactor. Each supported sentence type has its
own independently green commit. Its tests cover exact wire output, checksum,
line ending, optional fields, relevant invalid values, and parsing through
`updraft_nmea::parse()`.

Replay tests cover these behaviors:

- Existing NMEA replay remains byte-for-byte identical.
- `.nmea` and `.igc` matching is case-insensitive.
- Missing and unsupported extensions are rejected.
- The representative WeGlide IGC fixture produces timed, parseable NMEA.
- Missing extensions leave their NMEA fields empty.
- Zero altitudes are absent.
- Missing dates still produce RMC.
- Midnight increments the replay time and RMC date.
- B and K data merge only at an equal timestamp.
- LXWP1 occurs at time zero and every 60 seconds.
- Malformed mapped records produce warnings and do not stop usable replay.
- A file without a usable B record is rejected.

The server test continues to prove background playback, late-client behavior,
and simultaneous clients. It does not repeat IGC or NMEA field tests.

## Validation

The change must pass the focused crate tests after each red-green-refactor
cycle. The completed change must pass these continuous integration checks:

```bash
cargo fmt --all --check
cargo clippy --workspace --exclude updraft_tauri --exclude tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo test --workspace --exclude updraft_tauri --exclude tauri-plugin-updraft --all-features
cargo doc --workspace --exclude updraft_tauri --exclude tauri-plugin-updraft --no-deps --all-features
```

# Device I/O

All device I/O (serial, Bluetooth, BLE, TCP/UDP) sits behind Rust traits. Production code plugs in real adapters. Tests plug in fakes.

Sensor data reaches the core from three source categories:

- **external devices**, connected through transports (this document),
- **internal sensors** of the phone/tablet (this document),
- **simulator/replay**, which disables the other two while active (see [simulator.md](simulator.md)).

A _device_ in this document is always a connected instrument (GPS, vario, FLARM). A copilot's phone or tablet is a _client_, covered in [multi-client.md](multi-client.md).

## Transports

A **transport** is a byte stream in/out. Implementations:

- Bluetooth SPP (Android, desktop)
- BLE serial, Nordic-UART-style (Android, iOS, desktop)
- TCP client & server
- UDP
- USB-serial via Android OTG
- serial/USB on desktop
- replay from recorded byte captures (developer mode only, see [devmode.md](devmode.md))

Serial transports carry a per-connection baud configuration.

The platform-specific sides (Bluetooth plugins, foreground service) live in the Tauri shell (see [tauri.md](tauri.md)), but they all feed the core through this same abstraction. The core never knows whether an NMEA stream came from SPP, BLE, TCP, a serial port, or a replay file.

## Internal Sensors

Internal sensors (GPS, pressure sensor) skip the transport and parser machinery entirely: they produce typed messages directly onto the core's input queue. In the source priority they always rank below external devices. The platform-specific wiring lives in the Tauri shell (see [tauri.md](tauri.md)).

A vario derived from the internal pressure sensor is not compensated for airspeed changes (no total-energy compensation), so it can only ever serve as a backup for a real vario device.

Internal sensors stay active by default even while external devices provide the same data (a WeGlide-valid IGC log requires continuous internal GPS input). A single battery-saver setting suspends internal acquisition while external devices actively provide the same data.

## Parsing

External device data (NMEA sentences, vendor protocols, binary frames) is parsed by pure functions — bytes in, typed messages out — with no knowledge of the transport that carried them. There is no separate framer, dispatcher, or parser registry: a single parser per _framing_ owns the whole job (locating frame boundaries, validating checksums, resynchronising, decoding) and always decodes every sentence family it knows. Nothing is enabled, disabled, or routed per stream.

Two framings are supported in the target design:

- **line-based text** (`updraft_nmea`) — NMEA-style sentences delimited by line terminators, covering every text family that shares this framing: generic GNSS (any talker ID, with the nonstandard `$BD` BeiDou talker aliased), and the vendor families (Garmin `$PGRMZ`, FLARM `$PFLA*`, LXNav `$LXWP*`/`$PLXV*`, OpenVario `$POV`, Cambridge `!w`, …). They interleave freely on one stream — an LX9000 emits GNSS, baro, FLARM pass-through and both LXNav families on one port — so they share one parser.
- **GDL90** (`updraft_gdl90`) — the flag-delimited binary ADS-B framing, on its own transport. Added when ADS-B input lands; the read path is structured so it slots in as a second framing without touching the text parser or its callers.

The parser is a function the connection _calls_; it never owns the connection. On each step it yields exactly one of:

- **`Frame`** — a complete, checksum/CRC-validated frame, decoded into a wire-faithful typed message;
- **`Incomplete`** — the buffer does not yet hold a full frame; it is retained and the caller feeds more bytes;
- **`Rejected`** — the bytes at the front are not a valid frame (bad checksum, corruption, junk); the parser discards forward to the next frame boundary, records which kind of failure it was, and retries.

Framing is boundary-based (start marker to terminator), which is what makes `Incomplete` and `Rejected` fall out naturally and lets a connection that joins mid-stream self-heal at the next clean boundary. A well-framed sentence whose type is unrecognised is not `Rejected` — it is a `Frame` carrying an `Unknown` message, counted and logged, never silently dropped.

The parsers are **hand-rolled**, not built on a parser-combinator or regex library. NMEA is flat and line-oriented, so the genuinely non-trivial parts (streaming framing, resync, failure tallying) are owned directly regardless of tooling, while field parsing is plain delimiter splitting into typed quantities. Hand-rolling keeps a zero-dependency, easily-fuzzed crate on the untrusted-input boundary. The binary GDL90 framing may revisit this when it lands.

### Wire messages and core input

Parser output is **wire-faithful**: one typed message per sentence, every wire field represented and parsed into typed quantities (via `updraft_units`), but with no device-specific fix-ups. A device that reports LX wind with a flipped direction is parsed verbatim; the correction is a per-device config flag applied _downstream_ of the parser, never inside it. This keeps the parser a pure, losslessly-testable function.

Between the parser and the core's input queue, parsed messages are normalised and per-device corrections applied, then delivered as the typed messages the core consumes (the same queue internal sensors and replay feed). The core's input shape is **semantic messages carrying provenance**: vendor-agnostic observations (position, pressure altitude, traffic, …) tagged with the source device ID and a raw-sentence reference, so the reducers stay vendor-agnostic while devmode diagnostics and the capability observer retain full fidelity — vendor wire shapes never cross into the core. Multi-device merging and source priority operate on the same per-category view, so they never need to know which framing or vendor produced a value.

Some operations switch a device out of normal parsing entirely: a driver can claim the stream for an **exclusive binary session** for the duration of an operation such as a flight-log download (FLARM's binary IGC protocol being the canonical case), after which normal framing and parsing resume. This mode is part of the target design; because the parser is called rather than owning the connection, the connection simply stops feeding the parser and routes raw bytes to the session for its duration. The detailed protocol is deferred.

## Drivers & Personalities

A device _driver_ is device-specific knowledge layered on a shared transport. Its _read_ side — which sentence families to understand — is no longer a separate thing: the parser already decodes every family, and which families a device actually emits is observed, not configured (see [Auto-Detection](#auto-detection)). What remains is the driver's _write_ side, an optional **device personality**.

A personality keeps settings such as MacCready, ballast, bugs, and QNH in **bidirectional sync**: turn the knob on the vario and the app follows, change the value in the app and the device follows. The most recent change wins, regardless of where it was made. Each device carries per-setting sync preferences (send, receive, both, or neither), with full sync as the default. Beyond settings sync, personalities handle one-shot outbound operations: task declarations and config commands. For LXNav hardware the sync channel is the `$PLXV*` settings protocol. Which personality speaks is auto-detected from the observed message stream, with a manual override for the protocol used to send.

The personality layer is part of the target design; its detailed design — the outbound protocols, and the probe queries that wake silent request/response devices during identification — is deferred.

## Auto-Detection

Detection has two independent layers: which _framing_ a connection speaks, and which _capabilities_ its messages reveal.

**Framing selection.** A new connection runs its incoming bytes through the candidate framings and watches the `Frame`/`Rejected` tally; the framing that yields valid, checksum-passing frames wins, one that yields only `Rejected` loses. This is the same principle as the serial **baud probe** (cycle common rates until valid checksums appear), one level up: on a serial transport the two nest into a baud-by-framing sweep. Manual override of framing and baud remains available for unusual or silent hardware. Today there is a single text framing; GDL90 joins as a second candidate in the same sweep without changing the selector.

**Capability observation.** Above the parser sits a passive observer that never changes what the parser does — it only watches the typed messages flow past and tags the connection: valid position fixes → "GPS source", baro-altitude sentences → "pressure altitude", traffic sentences → "FLARM traffic", the LXNav families → "LXNav vario, protocol vX". A combo unit accumulates all its tags simply because those messages appear; nothing was enabled to make it happen. The tags drive:

1. the source priority (see below),
2. which **device personality** attaches,
3. what the **devices screen** shows ("LX9000 — GPS ✓ Vario ✓ Traffic ✓").

The active side of identification — sending probe queries to wake silent request/response devices or read current settings — is outbound and belongs to the deferred personality layer; it fires during this same identification window.

When a connection drops, the platform side reconnects with backoff (see [tauri.md](tauri.md)). A reconnected stream resumes its previously selected framing and capability tags instantly from session state, so a flapping link never yanks a device out of the priority order; detection re-runs in the background and revises the tags if the hardware changed. After an app restart, detection starts fresh.

## Source Priority

External devices form a single, user-ordered priority list. For each data category (position, pressure altitude, vario, traffic, …), the highest-ranked device currently providing fresh data wins; internal sensors are the fallback below all external devices. A source counts as **stale** when it hasn't delivered valid data within a per-category threshold.

The list itself only changes when the user reorders it. A stale source keeps its slot: when it delivers again, it automatically takes precedence again. Every fallback is announced to the pilot ("LX9000 disconnected, falling back to internal GPS").

## Device Configs

What persists per device is deliberately slim: the transport configuration (Bluetooth address, TCP host/port, serial port and baud), manual overrides, and the per-setting sync preferences. Detection results are session-only state.

A **device config** is a named, saveable snapshot of the whole setup: the device entries and their priority order. Loading a device config replaces the entire current list. An aircraft config can reference a device config, so switching aircraft automatically connects to that aircraft's panel. A config doesn't have to be attached to an aircraft: a pilot who carries an LX Nano across gliders saves a device config for it and loads it manually.

The devices screen manages all of this: connection status per device, priority reordering, manual overrides, and config save/load.

## Testing

Framing, parsing, and detection heuristics are pure functions over byte sequences, tested against recorded captures from real devices and fuzzed to never panic on arbitrary bytes (see [testing.md](testing.md)). Each vendor family within `updraft_nmea` carries its own proptest no-panic suite and snapshot tests against the shared `testdata/` corpus, so the families harden independently despite sharing a crate.

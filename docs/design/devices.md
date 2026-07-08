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

Internal sensors (GPS, pressure sensor) skip the transport and parser machinery entirely: they produce typed messages directly onto the core's input channel. In the source priority they always rank below external devices. The platform-specific wiring lives in the Tauri shell (see [tauri.md](tauri.md)).

A vario derived from the internal pressure sensor is not compensated for airspeed changes (no total-energy compensation), so it can only ever serve as a backup for a real vario device.

Internal sensors stay active by default even while external devices provide the same data (a WeGlide-valid IGC log requires continuous internal GPS input). A single battery-saver setting suspends internal acquisition while external devices actively provide the same data.

## Parser Stack

The parsing of external data (NMEA sentences, vendor protocols, file formats) lives in pure functions (bytes in, typed values out) that are trivially unit-testable, separate from the transport that carried the bytes.

Two pure pieces make up the NMEA library (`updraft_nmea`):

- a **framer** that splits a byte buffer into sentences and verifies the NMEA checksum (NMEA lines first, including line records that don't start with `$` such as the Cambridge CAI302's `!w` vario records, extensible to binary frames), and
- a **single parse entry point**, `parse(sentence) -> ParseResult` — deliberately *not* a registry of parsers. `ParseResult` is one enum covering every sentence family the crate recognizes:
  - standard GNSS `GGA` / `RMC` / `GSA` (routed by the three-letter sentence formatter, accepting any GNSS talker ID; the nonstandard `$BD` BeiDou talker is treated as an alias)
  - FLARM `PFLAU` / `PFLAA`
  - Garmin `$PGRMZ`
  - LXNav `$LXWP*` and `$PLXV*` (settings read/write, declarations, log transfer)
  - an `Unsupported` variant for a well-formed, checksum-valid sentence the crate does not model

Internally each sentence is its own pure function so it can be unit-tested and fuzzed in isolation; `parse` is a thin router over them. A device such as an LX9000 that emits NMEA, `$PGRMZ`, FLARM pass-through, and both LXNav families on one port is handled by this one function — each line simply returns whichever `ParseResult` variant it matched, so no per-stream parser wiring is needed.

Routing, identification, and capability tagging are a **device-layer concern, not a parser-crate one**, and land with `io-adapters`. That layer feeds transport bytes through the framer, calls `parse`, routes the resulting typed values into the core, and derives capability tags ("GPS source", "FLARM traffic") from which `ParseResult` variants a stream actually produces. It can start as a plain `match`; a runtime parser registry is introduced only if pluggability — third-party or user-configurable parsers competing for the same bytes — ever proves necessary, which the current device set does not require.

Some operations switch a device out of NMEA entirely: a driver can claim the stream for an **exclusive binary session** for the duration of an operation such as a flight-log download (FLARM's binary IGC protocol being the canonical case), after which normal framing and dispatch resume.

## Drivers & Personalities

A device _driver_ is the device-specific knowledge: which sentence families to parse and which personality to speak, layered on top of a shared transport. Crucially, **a driver is deliberately not the owner of a connection.** It is a sentence-family handler plus an optional _device personality_.

A personality keeps settings such as MacCready, ballast, bugs, and QNH in **bidirectional sync**: turn the knob on the vario and the app follows, change the value in the app and the device follows. The most recent change wins, regardless of where it was made. Each device carries per-setting sync preferences (send, receive, both, or neither), with full sync as the default. Beyond settings sync, personalities handle one-shot outbound operations: task declarations and config commands. For LXNav hardware the sync channel is the `$PLXV*` settings protocol.

## Auto-Detection

New connections start in an **identification mode** with all parsers enabled promiscuously. Identification is not purely passive: drivers can register probe queries that are sent during the identification window (reading an LXNav's current settings, waking request/response devices). On serial transports a **baud probe** runs first, cycling common rates until valid checksums appear; manual baud configuration remains available.

After a short observation window (or immediately on a signature sentence), the stream is tagged with detected capabilities ("GPS source", "FLARM traffic", "LXNav vario, protocol vX"), which drives:

1. the source priority (see below),
2. which **device personality** attaches,
3. what the **devices screen** shows ("LX9000 — GPS ✓ Vario ✓ Traffic ✓").

Manual override remains available for unusual hardware.

When a connection drops, the platform side reconnects with backoff (see [tauri.md](tauri.md)). A reconnected stream resumes its previously detected capabilities and personality instantly from session state, so a flapping link never yanks a device out of the priority order; identification re-runs in the background and revises the tag if the hardware changed. After an app restart, identification starts fresh.

## Source Priority

External devices form a single, user-ordered priority list. For each data category (position, pressure altitude, vario, traffic, …), the highest-ranked device currently providing fresh data wins; internal sensors are the fallback below all external devices. A source counts as **stale** when it hasn't delivered valid data within a per-category threshold.

The list itself only changes when the user reorders it. A stale source keeps its slot: when it delivers again, it automatically takes precedence again. Every fallback is announced to the pilot ("LX9000 disconnected, falling back to internal GPS").

## Device Configs

What persists per device is deliberately slim: the transport configuration (Bluetooth address, TCP host/port, serial port and baud), manual overrides, and the per-setting sync preferences. Detection results are session-only state.

A **device config** is a named, saveable snapshot of the whole setup: the device entries and their priority order. Loading a device config replaces the entire current list. An aircraft config can reference a device config, so switching aircraft automatically connects to that aircraft's panel. A config doesn't have to be attached to an aircraft: a pilot who carries an LX Nano across gliders saves a device config for it and loads it manually.

The devices screen manages all of this: connection status per device, priority reordering, manual overrides, and config save/load.

## Testing

Framing, dispatch, parsing, and detection heuristics are pure functions over byte or sentence sequences, tested against recorded captures from real devices and fuzzed to never panic on arbitrary bytes (see [testing.md](testing.md)).

# updraft_condor

Status: Planned design. Steps 1 to 3 of the implementation plan are complete.
The UDP sections describe planned behavior.

`updraft_condor` is a development tool that runs on the Windows PC that runs
the Condor 3 soaring simulator. It merges the three Condor outputs into one
NMEA stream and serves that stream over TCP. Updraft connects with its
existing TCP external device and needs no protocol change.

The [Condor 3 outputs investigation](../../docs/research/investigations/2026-09-11-condor-3-outputs.md)
records the evidence for the formats described here.

## Data flow

| Input | Transport | Content used |
| --- | --- | --- |
| Condor NMEA | TCP listener, HW VSP3 connects | `$GPGGA`, `$GPRMC`, `$LXWP0` |
| Condor UDP | UDP socket, Condor sends | `MC` |
| `Spectate.json` | File watcher | All players in the multiplayer race |

The output is one TCP listener. Every connected client receives the same
sentences:

| Sentence | Source | Purpose in Updraft |
| --- | --- | --- |
| `$GPGGA`, `$GPRMC`, `$LXWP0` | NMEA passthrough | Position, time, MSL altitude, TAS, wind |
| `$PGRMZ` | Derived from `$LXWP0` | Pressure altitude, vario, average vario |
| `$LXWP2` | Derived from UDP `MC` | MacCready, when Updraft consumes it |
| `$PFLAA`, `$PFLAU` | Derived from `Spectate.json` | Traffic on the map |

The tool keeps a small state: the latest own position from NMEA, the latest
MacCready value, the own-ship identity, and the last emitted traffic snapshot.
It has no user interface beyond the console.

## NMEA input

HW VSP3 runs as a TCP client, as in the Condor manual's XCSoar recipe, and
delivers the bytes that Condor writes to the virtual COM port. The tool does
not open serial ports.

The listener accepts one connection at a time. A new connection replaces the
old one, so a restarted HW VSP3 recovers without a tool restart. A dropped
connection pauses the output stream.

The stream is split into sentences with the framing code in `updraft_nmea`.
Sentences with a valid checksum pass through unchanged. Invalid or truncated
lines are dropped and logged at debug level. Output lines end in CRLF.

The tool decodes three sentences for its own state:

- `$GPGGA` and `$GPRMC` give the own position, MSL altitude, and UTC time.
  The latest values and their receive time are the fallback reference for
  traffic.
- `$LXWP0` gives the baro altitude field. The tool emits it as `$PGRMZ` in
  feet directly after the original sentence. Updraft accepts pressure
  altitude only from `$PGRMZ`, derives vario from it, and its filter is tuned
  for this 1 Hz cadence.

Condor's `$GPGGA` has no geoid separation and Updraft reads its altitude as
MSL, which matches Condor. Condor's `$GPRMC` has no date and Updraft stores a
time-only fix time, which is enough for freshness.

After ten seconds without a sentence the tool logs one warning. The NOTAM "No
PDA" option and gliders without a PDA are the usual causes.

## Spectate input

The tool watches the folder that contains `Spectate.json` with the `notify`
crate, because the file does not exist before a multiplayer session starts.
Change events are debounced for 50 ms. The tool then reads the file, strips
a UTF-8 byte order mark, and parses a JSON array of player objects. A parse
failure means Condor was still writing, so the tool waits for the next event.
Content identical to the previous read emits nothing.

Each player object has string values. The relevant fields are `ID`, `CN`,
`RN`, `latitude`, `longitude`, `altitude` in metres, `speed` in km/h,
`heading` in degrees, and `vario` in m/s. Coordinates are a hemisphere letter
followed by decimal degrees, such as `N45.000000` and `E013.000000`.

The own entry is found by competition number, compared case-insensitively
after trimming, then by registration. Its position and altitude are the
reference for the other players in the same snapshot. Without an own entry the
tool uses the NMEA position and GGA altitude when they are less than three
seconds old. Without either reference the snapshot emits nothing.

For every other player the tool emits one `$PFLAA`:

- alarm level `0`
- relative north and east in whole metres from the geodesic distance and
  bearing
- relative vertical in whole metres, when both altitudes are known
- ID type `2` and a 24-bit identity from the numeric `ID` masked to 24 bits,
  with a hash of the text as fallback, followed by `!` and the competition
  number
- heading as track, speed in whole m/s, and vario as climb rate with one
  decimal
- aircraft type `1` for glider

One `$PFLAU` with the traffic count closes each snapshot. The tool sends
traffic only when the file changes. Updraft marks targets stale by its own
rule when the session ends.

The tool never synthesizes `$GPGGA` or `$GPRMC` from the own entry. When Condor
withholds the PDA, the tool does not supply a position through the JSON.

## UDP input

The tool binds a UDP socket on the address from `UDP.ini`, default
`127.0.0.1:55278`. Condor sends the datagrams. Each datagram is ASCII text with
one `key=value` pair per line. The first version reads only `MC` in m/s,
which needs `ExtendedData1=1`. All other keys are ignored.

The value is emitted as `$LXWP2` with MacCready in the first field and every
other field empty. The tool sends it when the value changes, rate-limited to
once per second, and every five seconds regardless, so a client that connects
during a flight receives the current value.

## Output

The output listener defaults to `0.0.0.0:4354` and broadcasts to every
connected client. A client that falls behind is dropped, following
`updraft_replay`. Windows Firewall asks once for the listener.

## Configuration

The config file is `updraft_condor.ini` next to the executable. The tool
writes it on first run with every effective value and a comment per line. A
value in the file wins over detection.

```ini
[Condor]
; Condor 3 installation folder. Detected when empty.
Folder=
; Own competition number for the Spectate file. Detected from the pilot profile when empty.
CompetitionNumber=

[Input]
; HW VSP3 connects here with the NMEA stream from Condor.
NmeaListen=127.0.0.1:4353
; Must match Host and Port in Condor's UDP.ini.
UdpListen=127.0.0.1:55278

[Output]
; Updraft connects here. 0.0.0.0 accepts connections from other devices.
NmeaListen=0.0.0.0:4354
```

Detection runs for the two Condor values. The folder comes from the default
install location, confirmed by its `Settings` subfolder. The competition number
comes from `pilot.ini` in the single profile folder under the user's
Documents. When the folder is missing or more than one profile exists, the tool
asks for that value on the console and writes the answer into the config
file. A `--config` flag selects another file for development.

## Startup and console

At startup the tool prints the detected folder and competition number, the
address HW VSP3 must connect to, the UDP address Condor sends to, the path of
the Spectate file, and the output address with the machine's LAN addresses.

It then reads `UDP.ini` from the Condor `Settings` folder. If output is
disabled or `ExtendedData1` is off, it says so and offers to write the
required lines after a yes on the console. Condor reads the file at the next
flight.

Logging uses `tracing` on the console. Lifecycle events are at info level:
client connected, Condor NMEA started, Spectate file found, and their
counterparts. Per-sentence detail is at debug level and off by default.

The window stays open while the tool runs. On a fatal error, such as a port
already in use, the tool prints the cause and waits for Enter before it
closes.

## Setup

1. Install HW VSP3. Create a virtual COM port in TCP client mode with the
   tool's NMEA input address.
2. In Condor, enable NMEA output on that COM port under Setup > Options.
3. Enable UDP output with `ExtendedData1=1` in `UDP.ini`, or let the tool do
   it.
4. Start the tool. Copy the output address from the console.
5. In Updraft, add a TCP external device with that address.

## Code structure

- `config.rs` owns the INI file, detection, and prompts.
- `nmea.rs` validates passthrough lines and derives `$PGRMZ` from `$LXWP0`.
- `spectate.rs` converts one parsed snapshot and a reference position into
  `$PFLAA` and `$PFLAU` lines. It is a pure function.
- `udp.rs` extracts `MC` and produces `$LXWP2`.
- `server.rs` is the broadcast TCP server, lifted from `updraft_replay`.
- `main.rs` wires the tasks on one Tokio runtime.

The crate depends on `updraft_nmea` for framing, checksums, and the sentence
encoder, and on `updraft_units` for conversions. It does not depend on
`updraft_core`.

## Tests

- Insta snapshots for the sentence output of a fixed JSON snapshot, including
  the `XCSoar` test fixture, and for a passthrough sample with its derived
  `$PGRMZ`.
- Unit tests for coordinate parsing, the flat-earth offsets, the 24-bit
  identity, the rate limit, and config precedence.
- `testdata/condor/` holds a `Spectate.json` sample and a UDP datagram
  capture.
- No test opens sockets or watches real folders.

## Excluded behavior

- Serial port access. HW VSP3 owns the COM port.
- Ballast. Condor reports water in kilograms and the LX sentence needs an
  overload factor, which requires the glider reference mass.
- UDP attitude, rates, varios, and g-load. Updraft does not consume them yet.
- Own position from the Spectate file.
- Discovery of the PC address from Updraft.

## Open checks

These need a Condor 3 installation:

- The exact `Spectate.json` location and coordinate string format in 3.1.0.
  XCSoar uses `c:\condor3\logs\spectate.json` and `N45.000000`.
- The keys in `pilot.ini`.
- Whether the NOTAM "No PDA" option also stops the Spectate file.
- Whether Condor rewrites the file in place or by rename, and how often.

## Implementation plan

Each step is one reviewable commit with its tests.

1. Config file, detection, prompts, and the startup summary.
2. Output server and NMEA input with passthrough and the `$PGRMZ` derivation.
3. Spectate snapshot conversion, the file watcher, and own-ship reference
   selection.
4. UDP input with `$LXWP2`.
5. The `UDP.ini` check and offer to write it.
6. A Windows build in CI and a release artifact.

# Replay Server

Status: Current development tool

`updraft_replay` serves recorded flight data as a timed NMEA byte stream over
TCP. It supports development and manual testing of Updraft device connections.
It is not an in-application replay mode.

## Run the server

Use an NMEA or IGC input file:

```console
cargo run -p updraft_replay -- testdata/nmea/basic.nmea
cargo run -p updraft_replay -- testdata/igc/basic.igc
```

The command accepts:

```text
updraft_replay [--listen <ADDRESS>] [--loop] [--skip <SECONDS>] <FILE>
```

The default address is `127.0.0.1:4353`. Configure an enabled TCP external
device in Updraft with the same address.

`--skip` starts the first pass at the specified elapsed replay time. It fails
when the value is greater than the replay duration. `--loop` restarts later
passes from the beginning.

The server starts its replay clock after it binds the TCP listener. A client
receives events from the current replay position after it connects. Multiple
clients receive the same later events. A slow or disconnected client does not
stop playback for other clients.

## NMEA input

An `.nmea` file keeps its original bytes. Valid RMC or GGA UTC timestamps divide
the bytes into replay events. The first timestamp defines replay time zero.

The file must contain at least one valid timestamp. Midnight advances the
timeline by one day. A smaller backward timestamp change stays at the current
replay position and produces one warning.

## IGC input

An `.igc` file converts supported IGC records and extensions to NMEA sentences.
The file must contain at least one usable B record. The first usable timestamp
defines replay time zero.

The conversion can produce RMC, GGA, PGRMZ, LXWP0, LXWP1, and PLXVS sentences.
Available IGC fields determine which values appear. Unsupported records do not
produce replay data. Invalid mapped records and fields produce warnings when a
usable replay can continue.

IGC playback uses the same monotonic timeline rules as NMEA playback. It also
updates the RMC date across midnight when the IGC input supplies a date.

## Excluded behavior

The tool does not control Updraft, seek after startup, change playback speed,
record device input, or provide a user interface. It does not implement the
planned simulator or user-facing flight replay features.

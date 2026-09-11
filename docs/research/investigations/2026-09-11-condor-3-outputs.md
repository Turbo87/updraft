# Condor 3 data outputs

This investigation records which data the Condor 3 soaring simulator can send
to external software, and in which form. It informs a possible Condor input
source for Updraft. It does not define Updraft behavior.

Examined version: Condor 3, user guide 1.03 for version 3.0.9, with forum
reports up to version 3.1.0 (released 2026-01-26). Platform: Windows only.
Date of research: 2026-09-11. No Condor installation was available, so every
observation comes from the manual, the Condor forum, and third-party source
code that consumes the outputs.

## Conclusion

Condor 3 has three separate outputs. No single output contains everything.

| Output | Transport | Own position | Rate | Other content |
| --- | --- | --- | --- | --- |
| NMEA | Serial COM port | Yes | 1 Hz | TAS, altitude, vario, wind |
| UDP | UDP datagrams | No | Configurable, down to 1 ms | Attitude, rates, varios, MC, water |
| Spectate JSON | File on disk | Yes, with all players | About 1 Hz | Names, competition numbers, scores |

- NMEA is the only stream with a documented own-ship position and time.
- UDP is the only stream with attitude, body rates, and high update rates.
- The Spectate JSON is the only source of other players. Older versions served
  it over local HTTP. Versions from 3.0.3 write a file instead.
- None of the outputs contain FLARM traffic sentences or airspace data.

## NMEA output

Setup > Options has an NMEA output checkbox and a COM port selection. The
manual describes it as a connection for "a Palm, PocketPC or other navigation
hardware that supports NMEA". The Condor team confirmed on the forum that
Condor 3 sends `$GPGGA`, `$GPRMC`, and `$LXWP0`, and that the set can grow
"if needed".

Example sentences recorded from Condor and posted by users:

```text
$GPGGA,120009.136,4621.5849,N,01410.2437,E,1,12,10,504.7,M,,,,,0000*00
$GPRMC,120009.136,A,4621.5849,N,01410.2437,E,0.00,134.00,,,,*20
$LXWP0,Y,0.1,504.7,0.00,,,,,,134,351,19.5*71
$LXWP0,Y,104.9,1397.8,-1.15,,,,,,78,017,17.0*55
```

Observed properties:

- Sentences arrive once per second. A forum user reported that the rate is
  fixed and that the selected baud rate does not change it. Pausing Condor
  stops the output.
- `$GPGGA` contains UTC time with milliseconds, position, fix quality `1`,
  12 satellites, HDOP `10`, and MSL altitude in metres. The geoid separation
  field and the differential fields are empty.
- `$GPRMC` contains time, status `A`, position, ground speed in knots, and
  true track. The date field is empty.
- `$LXWP0` fields are: logger flag, true airspeed in km/h, barometric
  altitude in metres, total-energy vario in m/s, six empty fields, heading in
  degrees, wind direction in degrees, and wind speed in km/h. The XCSoar
  driver notes that Condor sends true airspeed in the field that LX devices
  use for indicated airspeed. The wind is the instantaneous simulator wind, not
  an averaged estimate.
- Condor 1 and 2 sent the direction the wind was going to. Condor 3.0.1
  changed `$LXWP0` to the direction the wind comes from, which matches LX
  devices. XCSoar therefore has a separate "Condor Soaring Simulator 3" driver
  that does not invert the wind. The maintainers state that nothing else
  changed in the serial protocol between Condor 2 and Condor 3.
- Positions are already converted from the landscape grid to WGS84 latitude
  and longitude. Condor landscapes use a UTM grid, and the `NaviCon.dll`
  library in the installation folder performs the conversion. Old forum
  reports mention alignment errors of up to several kilometres on older
  landscapes because of per-landscape calibration. No current measurement of
  this error was found.

Conditions that disable the output:

- The NOTAM "No PDA" option disables NMEA output for that flight.
- Gliders without an in-game PDA, such as the Grunau Baby, SG38, and Swift,
  never produce NMEA output. The UDP stream still works for them.

Condor writes an `NMEAlog.txt` file in the `Logs` folder of the installation
for debugging. A user reported that the file appears without any setting.

Requested but not available in Condor 3.1.0: `$PFLAA` and `$PFLAU` traffic
sentences, `$PLXV0` MacCready commands, `$LXWP2`, and task transfer. The Condor
team answered "not sent via NMEA (yet)" in October 2024.

The manual describes two setups for external flight computers. A virtual
serial port such as HW VSP3 or com0com with hub4com forwards the COM port to
a TCP port on the network. A Bluetooth COM port on the PC works with a
Bluetooth SPP client. Users also bridge the data with a Python RFCOMM server
for Android devices.

## UDP output

The manual documents the generic UDP output in section 14.1. A `UDP.ini` file
in the `Settings` folder enables it:

```ini
[General]
Enabled=1
[Connection]
Host=127.0.0.1
Port=55278
[Misc]
SendIntervalMs=1
ExtendedData=0
ExtendedData1=0
LogToFile=0
```

Condor sends datagrams to the configured host and port. It does not listen
for clients. `SendIntervalMs` sets the time between datagrams.
`LogToFile=1` writes the same content to `UDPlog.txt` in the `Logs` folder.
Output only runs during a flight, not in the menus.

Each datagram is ASCII text with one `parameter=value` pair per line. Values
are floating-point numbers with a dot as the decimal separator. The Condor
team confirmed that all values use SI units in Condor 3. Condor 2 sent
altitude in the selected display unit. The manual for 3.0.9 still lists
"m or ft" for altitude, and the team stated that this is a manual error.

Parameters from the manual, with the additional keys observed in real packets:

| Parameter | Value | Unit | Availability |
| --- | --- | --- | --- |
| `time` | In-game display time | decimal hours | Always |
| `airspeed` | True airspeed | m/s | Always, missing from the manual table |
| `altitude` | Altimeter reading | m | Always |
| `vario` | Pneumatic vario | m/s | Always |
| `evario` | Electronic (total-energy) vario | m/s | Always |
| `nettovario` | Netto vario | m/s | Always |
| `integrator` | Integrator (averager) | m/s | Always |
| `compass` | Compass reading | degrees | Always |
| `slipball` | Slip ball deflection | rad | Always |
| `turnrate` | Turn indicator | rad/s | Always |
| `yawstringangle` | Yaw string angle | rad | Always |
| `radiofrequency` | Active radio frequency | MHz | Always, missing from the manual table |
| `yaw`, `pitch`, `bank` | Body attitude | rad | Always |
| `quaternionx`, `quaterniony`, `quaternionz`, `quaternionw` | Attitude quaternion | none | Always, `w` is missing from the manual table |
| `ax`, `ay`, `az` | Acceleration vector | m/s² | Always, missing from the manual table |
| `vx`, `vy`, `vz` | Velocity vector | m/s | Always, the manual lists `vx` three times |
| `rollrate`, `pitchrate`, `yawrate` | Body angular rates | rad/s | Always |
| `gforce` | Load factor | none | Always |
| `height` | Height of the centre of gravity above ground | m | `ExtendedData=1` |
| `wheelheight` | Height of the wheel above ground | m | `ExtendedData=1` |
| `turbulencestrength` | Turbulence strength | none | `ExtendedData=1` |
| `surfaceroughness` | Surface roughness | none | `ExtendedData=1` |
| `hudmessages` | HUD text, entries separated by `;` | text | `ExtendedData=1` |
| `flaps` | Flap position index, 0 is most negative | integer | `ExtendedData1=1` |
| `MC` | MacCready setting | m/s | `ExtendedData1=1` |
| `water` | Water ballast content | kg | `ExtendedData1=1` |

A datagram recorded by a user from Condor 3 with both extended options:

```text
time=13.0619016567222
airspeed=34.1800956726074
altitude=1813.67102050781
vario=1.80288767814636
evario=1.81920945644379
nettovario=2.93230128288269
integrator=2.9955894947052
compass=127.981353759766
yawstringangle=0.00345267145894468
radiofrequency=123.5
yaw=0
pitch=0
bank=0
quaternionx=0.0593169406056404
quaterniony=-0.0445633232593536
quaternionz=0.943771302700043
quaternionw=0.322165846824646
ax=0
ay=0
az=0
vx=-31.434965133667
vy=-26.2350311279297
vz=2.96723532676697
rollrate=0.0158377848565578
pitchrate=0.0095204645767808
yawrate=-0.0253506116569042
gforce=1.03178053456344
height=492.190673828125
wheelheight=0
turbulencestrength=0.418789803981781
surfaceroughness=20
flaps=4
MC=0
water=148
```

Observed properties:

- The stream has no latitude, longitude, or UTC time. Users who tried to
  build an NMEA bridge from it stopped for this reason. The XCSoar driver
  accepts optional `latitude` and `longitude` keys in case a future Condor
  version adds them, but no version sends them.
- `yaw`, `pitch`, `bank`, `ax`, `ay`, and `az` were zero in the recorded
  packet while the quaternion and rates changed. The reason is unknown.
- A Condor team member described the velocity axes as x westward, y
  northward, z upward. XCSoar found the `vx` and `vy` track unreliable as an
  earth-frame vector and uses only their magnitude as ground speed.
- XCSoar inverts the `bank` sign, because Condor reports a right wing down as
  positive.
- A user reported that `hudmessages` does not display correctly and suspected
  a Condor bug.
- The glider type is not a documented parameter. A user stated that it is
  present in undocumented form. This was not verified.
- Third-party consumers include SimHub, PanelBuilder, Free Condor
  Instruments, Condor2Arduino, and the XCSoar `Condor3UDP` driver in
  XCSoar 7.45. XCSoar maps `airspeed`, `altitude`, the three varios,
  `compass`, `pitch`, `bank`, rates, `gforce`, `MC`, `water`, and
  `radiofrequency`, and ignores the rest.

## Spectate JSON

Spectate! mode joins a multiplayer server as an observer. The manual states
that Condor then "can act as a HTTP server on localhost, serving JSON files
with data about all pilots in the race". The default port is 8080. A
`Spectate.ini` file in the `Settings` folder changes it:

```ini
[General]
Port=8081
```

The manual lists two paths, `/selectedPilot` and `/allPilots`, and states that
the data is intended for OBS Studio overlays. Its example for one pilot is:

```json
{
  "ID": "3103807898",
  "CN": "JD",
  "RN": "F-CTJD",
  "firstname": "Jean-David",
  "lastname": "Thoby",
  "country": "France",
  "plane": "Ventus3-15",
  "latitude": "45.53.345N",
  "longitude": "013.53.071E",
  "altitude": "118",
  "speed": "70",
  "heading": "268",
  "vario": "0.02",
  "playerstatus": "Warmup",
  "rank": "1",
  "score": "0.0 p",
  "penalty": "0.0 p",
  "averagespeed": "—",
  "dist": "—",
  "time": "—"
}
```

Version history from the forum:

- Update 3.0.2 (2024-12-11) lists "spectate outputs scoreboard in json file
  format".
- In 3.0.3 the localhost HTTP endpoints stopped responding. The Condor
  developer responsible for the feature answered that "the latest version is
  not managing a webserver anymore but generate a JSON file on the drive". The
  manual for 3.0.9 still describes the HTTP server and has not been updated.
- XCSoar 7.45 (2026-08-15) added a `Condor3Spectate` driver that reads
  `Spectate.json` and presents the players as FLARM traffic. Its author used
  the example path `c:\condor3\logs\spectate.json`. This suggests the file
  lives in the `Logs` folder, but the location was not confirmed by the
  Condor team in any source found.

The XCSoar implementation documents the file shape that Condor 3 writes today:

- The file is a JSON array of player objects. The driver also accepts one
  object. It strips a UTF-8 byte order mark.
- Every numeric value is a JSON string. The driver parses `altitude`,
  `speed`, `heading`, and `vario` from strings.
- `latitude` and `longitude` are strings with a hemisphere letter and a
  number, such as `N45.000000` and `E013.000000` in the XCSoar test fixture.
  The manual example uses `45.53.345N` instead. The XCSoar parser accepts a
  leading hemisphere letter and then reads one decimal number, so it does not
  handle the manual format. The real current format was not verified.
- `speed` is in km/h. `altitude` is in metres and matches the Condor
  altimeter, which XCSoar treats as MSL without a geoid correction.
  `heading` is in degrees. `vario` is in m/s.
- `ID` is a numeric player identifier. `CN` is the competition number and
  `RN` the registration. XCSoar derives a 24-bit pseudo FLARM address from
  `ID`.
- The list contains the own player. XCSoar finds the own ship by matching a
  configured competition number, which implies that the file is also written
  while flying in a multiplayer race, not only while spectating. This was not
  confirmed against Condor directly.
- XCSoar polls the file once per second and rebuilds its traffic list from
  each read.

An earlier request for a dedicated-server position dump was not implemented.
The dedicated server has no documented data output.

## Eliminated sources

- Sniffing the multiplayer network protocol. It is undocumented, and a forum
  user reported that positions on the wire are landscape coordinates rather
  than geographic coordinates.
- The LXSim instrument in Condor 3 Pro. Its link to Condor is internal. No
  external port was found.
- Simkits output. It is a hardware protocol with four scaled instrument
  values and a separate `Simkits.ini` file.

## Limits

- No Condor 3 installation was used. Example data comes from users and from
  the XCSoar test fixtures. The NMEA examples predate Condor 3 and are
  consistent with the statement that only the wind direction changed.
- The JSON file location, its write interval, and its exact coordinate string
  format for version 3.1.0 need a check on a real installation.
- The manual for 3.0.9 has known errors in the UDP table and the JSON
  section. Prefer forum statements by the Condor team where they conflict.
- Condor listens on UDP port 63404 during a session. Its purpose was asked on
  the forum in June 2026 and not answered.

## Sources

- [Condor 3 user guide 1.03 for version 3.0.9](https://downloads3.condorsoaring.com/manuals/Condor%203%20manual_en.pdf),
  pages 21, 64 to 65, and 72 to 74.
- Condor forum threads:
  [Condor 3 Serial Protocol](https://www.condorsoaring.com/forums/viewtopic.php?t=22165),
  [wind direction now incorrect for xcsoar](https://www.condorsoaring.com/forums/viewtopic.php?t=22499),
  [C3: FLARM Traffic on NMEA out?](https://www.condorsoaring.com/forums/viewtopic.php?t=22216),
  [NMEA data stream unreliable?](https://www.condorsoaring.com/forums/viewtopic.php?t=19947),
  [No NMEA output](https://www.condorsoaring.com/forums/viewtopic.php?t=20056),
  [NMEA output for GrunauBaby](https://www.condorsoaring.com/forums/viewtopic.php?t=18180),
  [Connecting Condor to SeeYou phone app](https://www.condorsoaring.com/forums/viewtopic.php?t=23125),
  [UDP output](https://www.condorsoaring.com/forums/viewtopic.php?t=23152),
  [SimHub anyone?](https://www.condorsoaring.com/forums/viewtopic.php?t=22986),
  [Update 3.0.2 is now released](https://www.condorsoaring.com/forums/viewtopic.php?t=22649),
  [Spectator bug in 3.03](https://www.condorsoaring.com/forums/viewtopic.php?t=22780),
  [Request for dedicated server option: multiplayer data dump to file](https://www.condorsoaring.com/forums/viewtopic.php?t=19232),
  [Converting Lat/Lon to Condor Flight Plan XY Coordinates](https://www.condorsoaring.com/forums/viewtopic.php?t=12474),
  [Condor Listening at UDP port 63404](https://www.condorsoaring.com/forums/viewtopic.php?t=23690).
- XCSoar sources:
  [Condor.cpp](https://github.com/XCSoar/XCSoar/blob/master/src/Device/Driver/Condor.cpp),
  [Condor3UDP.cpp](https://github.com/XCSoar/XCSoar/blob/master/src/Device/Driver/Condor3UDP.cpp),
  [Condor3Spectate.cpp](https://github.com/XCSoar/XCSoar/blob/master/src/Device/Driver/Condor3Spectate.cpp),
  [pull request 2599](https://github.com/XCSoar/XCSoar/pull/2599),
  [pull request 3058](https://github.com/XCSoar/XCSoar/pull/3058),
  [issue 1612](https://github.com/XCSoar/XCSoar/issues/1612),
  [issue 2488](https://github.com/XCSoar/XCSoar/issues/2488),
  [XCSoar 7.45 release notes](https://xcsoar.org/news/2026/08/15/xcsoar-7-dot-45-released.html).
- [ogn-tracker issue 2](https://github.com/glidernet/ogn-tracker/issues/2)
  with recorded Condor NMEA sentences.
- [Condor2Arduino](https://github.com/docop/Condor2Arduino) and
  [CondorUDP2COM](https://github.com/kbobrowski/CondorUDP2COM) UDP consumers.

# Only a confirmed position jump or a new flight resets the wind

The wind belongs to the air mass, not to a GPS device. Updraft therefore keeps
the wind when the GPS source changes, when the selected GPS becomes
unavailable, when the pilot disables, edits, or deletes the selected GPS
device, and when the airspeed source changes. Only a confirmed position jump
or a new flight resets the fusion state, and with it the wind.

Without an airspeed source, the wind feeds derived airspeed, Vario, and Netto.
A reset on a source change removes these values until the next circle, for
example when a Bluetooth device reconnects or the internal GNSS takes over.

## Considered options

- A reset on each GPS source change protects against a change from a real GPS
  to a simulator. A position jump detects that case directly, and a source
  change alone does not make the wind wrong.
- A reset on a fix-UTC jump is not necessary, because the fusion uses ingest
  time.

## Consequences

- A position jump is a confirmed change of position that no glider can fly.
  One bad fix must not reset the wind, so the core confirms a candidate jump
  with the next fixes.
- A wind from before an outage stays until the wind filter expires it. The
  filter ages without measurements, so no separate reset is necessary.

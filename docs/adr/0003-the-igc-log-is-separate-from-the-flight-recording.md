# The IGC log is separate from the flight recording

A signed IGC file can contain only data from the internal GNSS receiver and the
internal barometer. The flight recording combines values from the selected
sources, which can be external devices. The IGC writer therefore keeps its own
log of internal sensor data, and the flight recording does not serve the IGC
export.

## Consequences

- An IGC B record needs UTC, the WGS84 position, fix validity, pressure
  altitude, and the GNSS altitude above the ellipsoid. All values in one record
  must be from within 0.1 s of the record time. The FXA, SIU, and ENL extensions
  are mandatory for approved recorders.
- The core has no input for an internal barometer. Its `Fix` type has no
  accuracy and no satellite count. The core converts the internal ellipsoid
  altitude to MSL when the fix arrives. The IGC log needs these values before
  conversion.

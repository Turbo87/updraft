import type { Instruments } from '$lib/protocol/generated/Instruments';

export const INFOBOX_INSTRUMENTS: Instruments = {
  gps: {
    position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
    altitudeMeters: 1230,
    groundSpeedMetersPerSecond: 31,
    trackDegrees: 24,
    fixTime: null,
    stale: false,
  },
  pressureAltitude: null,
  trueAirspeed: { metersPerSecond: 30, stale: false },
  terrainElevation: { meters: 465, stale: false },
  altitudeAgl: { meters: 780, stale: false },
  derived: {
    altitude: { altitudeMslMeters: 1245, stale: false },
    bank: { angleDegrees: 12, stale: false },
    vario: { metersPerSecond: 1.8, stale: false },
    netto: { metersPerSecond: 1.2, stale: false },
    wind: { directionDegrees: 248, speedMetersPerSecond: 5, stale: true },
    rawVerticalSpeed: null,
    verticalSpeed: null,
    averageVario: null,
    airspeed: null,
    heading: null,
    relativeVario: null,
  },
};

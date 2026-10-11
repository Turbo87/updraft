import type { GpsInstruments } from '#lib/protocol/generated/GpsInstruments.js';

import { describe, expect, it } from 'vitest';

import { deadReckon, groundMotion } from './dead-reckoning';

const EARTH_RADIUS_METERS = 6_371_008.8;
const position = { latitudeDegrees: 50, longitudeDegrees: 6 };

/** East and north offset of the dead-reckoned position from `position`, in meters. */
function offset(elapsedSeconds: number, motion: Parameters<typeof deadReckon>[1]) {
  let reckoned = deadReckon(position, motion, elapsedSeconds);
  let latitude = (position.latitudeDegrees * Math.PI) / 180;
  let east =
    (((reckoned.position.longitudeDegrees - position.longitudeDegrees) * Math.PI) / 180) *
    EARTH_RADIUS_METERS *
    Math.cos(latitude);
  let north =
    (((reckoned.position.latitudeDegrees - position.latitudeDegrees) * Math.PI) / 180) *
    EARTH_RADIUS_METERS;
  return { east, north, trackDegrees: reckoned.trackDegrees };
}

function gps(fields: Partial<GpsInstruments>): GpsInstruments {
  return {
    position,
    altitudeMeters: null,
    groundSpeedMetersPerSecond: 30,
    trackDegrees: 90,
    turnRateDegreesPerSecond: 10,
    fixTime: null,
    stale: false,
    ...fields,
  };
}

describe('deadReckon', () => {
  it('moves along the track without a turn rate', () => {
    let reckoned = offset(1, {
      speedMetersPerSecond: 30,
      trackDegrees: 0,
      turnRateDegreesPerSecond: 0,
    });

    expect(reckoned.east).toBeCloseTo(0, 9);
    expect(reckoned.north).toBeCloseTo(30, 9);
    expect(reckoned.trackDegrees).toBe(0);
  });

  it('moves east with the longitude scale of the latitude', () => {
    let reckoned = offset(0.5, {
      speedMetersPerSecond: 30,
      trackDegrees: 90,
      turnRateDegreesPerSecond: 0,
    });

    expect(reckoned.east).toBeCloseTo(15, 9);
    expect(reckoned.north).toBeCloseTo(0, 9);
  });

  it('follows a circular arc with the turn rate', () => {
    let reckoned = offset(1.5, {
      speedMetersPerSecond: 10,
      trackDegrees: 0,
      turnRateDegreesPerSecond: 20,
    });

    // A right turn of 30° on a radius of 10 m/s / (20°/s).
    let radius = 10 / ((20 * Math.PI) / 180);
    expect(reckoned.east).toBeCloseTo(radius * (1 - Math.cos(Math.PI / 6)), 9);
    expect(reckoned.north).toBeCloseTo(radius * Math.sin(Math.PI / 6), 9);
    expect(reckoned.trackDegrees).toBeCloseTo(30, 9);
  });

  it('stops after 1.5 seconds', () => {
    let motion = { speedMetersPerSecond: 30, trackDegrees: 0, turnRateDegreesPerSecond: 20 };

    expect(offset(3, motion)).toEqual(offset(1.5, motion));
    expect(offset(-1, motion)).toEqual(offset(0, motion));
    expect(offset(0, motion)).toEqual({ east: 0, north: 0, trackDegrees: 0 });
  });

  it('limits the turn rate to 30 °/s', () => {
    let motion = { speedMetersPerSecond: 30, trackDegrees: 0 };

    expect(offset(1, { ...motion, turnRateDegreesPerSecond: 60 })).toEqual(
      offset(1, { ...motion, turnRateDegreesPerSecond: 30 }),
    );
    expect(offset(1, { ...motion, turnRateDegreesPerSecond: -60 }).trackDegrees).toBeCloseTo(
      -30,
      9,
    );
  });
});

describe('groundMotion', () => {
  it('uses the ground speed, track, and turn rate', () => {
    expect(groundMotion(gps({}))).toEqual({
      speedMetersPerSecond: 30,
      trackDegrees: 90,
      turnRateDegreesPerSecond: 10,
    });
  });

  it('moves straight without a turn rate', () => {
    expect(groundMotion(gps({ turnRateDegreesPerSecond: null }))).toMatchObject({
      turnRateDegreesPerSecond: 0,
    });
  });

  it('moves from 2 m/s', () => {
    expect(groundMotion(gps({ groundSpeedMetersPerSecond: 2 }))).not.toBeNull();
  });

  it.each([
    { reason: 'no ground speed', fields: { groundSpeedMetersPerSecond: null } },
    { reason: 'no track', fields: { trackDegrees: null } },
    { reason: 'a ground speed below 2 m/s', fields: { groundSpeedMetersPerSecond: 1.9 } },
    { reason: 'a stale position', fields: { stale: true } },
  ])('does not move with $reason', ({ fields }) => {
    expect(groundMotion(gps(fields))).toBeNull();
  });
});

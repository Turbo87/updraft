import { describe, expect, it } from 'vitest';

import { calculateDistanceAndBearing } from './geographic-position';

describe('calculateDistanceAndBearing', () => {
  it('calculates geographic distance and initial true bearing', () => {
    expect(
      calculateDistanceAndBearing(
        { latitudeDegrees: 50, longitudeDegrees: 6 },
        { latitudeDegrees: 51, longitudeDegrees: 6 },
      ),
    ).toEqual({
      distanceMeters: expect.closeTo(111_195.08, 2),
      bearingDegrees: expect.closeTo(0, 6),
    });
  });

  it('normalizes a negative Turf bearing', () => {
    expect(
      calculateDistanceAndBearing(
        { latitudeDegrees: 0, longitudeDegrees: 0 },
        { latitudeDegrees: 0, longitudeDegrees: -1 },
      ).bearingDegrees,
    ).toBeCloseTo(270);
  });

  it('returns zero distance and bearing for coincident positions', () => {
    let position = { latitudeDegrees: 50.823, longitudeDegrees: 6.186 };

    expect(calculateDistanceAndBearing(position, position)).toEqual({
      distanceMeters: 0,
      bearingDegrees: 0,
    });
  });
});

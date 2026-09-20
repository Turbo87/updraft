import type { DerivedWindInstruments } from '$lib/protocol/generated/DerivedWindInstruments';
import type { SolarPositionInstruments } from '$lib/protocol/generated/SolarPositionInstruments';

import { describe, expect, it } from 'vitest';

import { hillshadeLighting } from './terrain-style';

describe('hillshadeLighting()', () => {
  it('keeps fixed lighting at the top-left of the viewport', () => {
    expect(hillshadeLighting('fixed')).toEqual({
      'hillshade-illumination-anchor': 'viewport',
      'hillshade-illumination-direction': 335,
    });
  });

  it.each([
    [270, false],
    [248, true],
  ] as const)('uses wind direction %s with stale=%s', (directionDegrees, stale) => {
    let wind: DerivedWindInstruments = { directionDegrees, speedMetersPerSecond: 5, stale };

    expect(hillshadeLighting('wind', { wind })).toEqual({
      'hillshade-illumination-anchor': 'map',
      'hillshade-illumination-direction': directionDegrees,
    });
  });

  it('uses fixed lighting when wind is unavailable', () => {
    expect(hillshadeLighting('wind', { wind: null })).toEqual(hillshadeLighting('fixed'));
  });

  it.each([
    [194.3, 39.9, false],
    [248, -4, true],
  ] as const)(
    'uses solar azimuth %s at elevation %s with stale=%s',
    (azimuthDegrees, elevationDegrees, stale) => {
      let solarPosition: SolarPositionInstruments = { azimuthDegrees, elevationDegrees, stale };

      expect(hillshadeLighting('sun', { solarPosition })).toEqual({
        'hillshade-illumination-anchor': 'map',
        'hillshade-illumination-direction': azimuthDegrees,
      });
    },
  );

  it('uses fixed lighting when the solar position is unavailable', () => {
    expect(hillshadeLighting('sun', { solarPosition: null })).toEqual(hillshadeLighting('fixed'));
  });
});

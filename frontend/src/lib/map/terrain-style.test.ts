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

  it('lights terrain from the meteorological wind direction', () => {
    let wind: DerivedWindInstruments = {
      directionDegrees: 270,
      speedMetersPerSecond: 5,
      stale: false,
    };

    expect(hillshadeLighting('wind', { wind })).toEqual({
      'hillshade-illumination-anchor': 'map',
      'hillshade-illumination-direction': 270,
    });
  });

  it('uses fixed lighting when wind is unavailable', () => {
    expect(hillshadeLighting('wind', { wind: null })).toEqual(hillshadeLighting('fixed'));
  });

  it('uses stale wind direction', () => {
    let wind: DerivedWindInstruments = {
      directionDegrees: 248,
      speedMetersPerSecond: 5,
      stale: true,
    };

    expect(hillshadeLighting('wind', { wind })).toEqual({
      'hillshade-illumination-anchor': 'map',
      'hillshade-illumination-direction': 248,
    });
  });

  it('lights terrain from the solar azimuth', () => {
    let solarPosition: SolarPositionInstruments = {
      azimuthDegrees: 194.3,
      elevationDegrees: 39.9,
      stale: false,
    };

    expect(hillshadeLighting('sun', { solarPosition })).toEqual({
      'hillshade-illumination-anchor': 'map',
      'hillshade-illumination-direction': 194.3,
    });
  });

  it('uses fixed lighting when the solar position is unavailable', () => {
    expect(hillshadeLighting('sun', { solarPosition: null })).toEqual(hillshadeLighting('fixed'));
  });

  it('uses a stale solar position', () => {
    let solarPosition: SolarPositionInstruments = {
      azimuthDegrees: 248,
      elevationDegrees: -4,
      stale: true,
    };

    expect(hillshadeLighting('sun', { solarPosition })).toEqual({
      'hillshade-illumination-anchor': 'map',
      'hillshade-illumination-direction': 248,
    });
  });
});

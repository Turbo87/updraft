import { describe, expect, it } from 'vitest';

import { convertAltitude, convertDistance, convertSpeed, convertVerticalSpeed } from './units.js';

describe('convertAltitude', () => {
  it('converts canonical meters to the selected altitude unit', () => {
    expect(convertAltitude(1_000, 'm')).toBe(1_000);
    expect(convertAltitude(1_000, 'ft')).toBeCloseTo(3_280.84);
  });
});

describe('convertDistance', () => {
  it('converts canonical meters to metric, imperial, and aviation units', () => {
    expect(convertDistance(18_520, 'km')).toBeCloseTo(18.52);
    expect(convertDistance(18_520, 'mi')).toBeCloseTo(11.508);
    expect(convertDistance(18_520, 'nm')).toBe(10);
  });
});

describe('convertSpeed', () => {
  it('converts canonical meters per second to metric, imperial, and aviation units', () => {
    expect(convertSpeed(10, 'km/h')).toBe(36);
    expect(convertSpeed(10, 'mph')).toBeCloseTo(22.369);
    expect(convertSpeed(10, 'kt')).toBeCloseTo(19.438);
  });
});

describe('convertVerticalSpeed', () => {
  it('converts canonical meters per second to the selected vertical speed unit', () => {
    expect(convertVerticalSpeed(1, 'm/s')).toBe(1);
    expect(convertVerticalSpeed(1, 'ft/min')).toBeCloseTo(196.85);
    expect(convertVerticalSpeed(1, 'kt')).toBeCloseTo(1.944);
  });
});

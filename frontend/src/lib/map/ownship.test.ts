import { describe, expect, it } from 'vitest';
import { ownshipFeature } from './ownship';

describe('ownshipFeature', () => {
  it('builds a point feature at the given position', () => {
    let feature = ownshipFeature({
      location: { latitude: 50.823, longitude: 6.186 },
      track: 45,
    });
    expect(feature.geometry).toEqual({ type: 'Point', coordinates: [6.186, 50.823] });
    expect(feature.properties?.track).toBe(45);
  });

  it('defaults the track to zero when unset', () => {
    let feature = ownshipFeature({ location: { latitude: 0, longitude: 0 }, track: null });
    expect(feature.properties?.track).toBe(0);
  });
});

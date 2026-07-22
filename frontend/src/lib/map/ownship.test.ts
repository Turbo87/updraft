import { describe, expect, it } from 'vitest';

import { ownshipFeature } from './ownship';

const position = {
  latitudeDegrees: 50.823,
  longitudeDegrees: 6.186,
};

describe('ownshipFeature', () => {
  it('builds a point feature at the given position', () => {
    let feature = ownshipFeature(position, { status: 'current', value: 45 });
    expect(feature.geometry).toEqual({ type: 'Point', coordinates: [6.186, 50.823] });
    expect(feature.properties?.track).toBe(45);
  });

  it('defaults the track to zero when unset', () => {
    let feature = ownshipFeature(position, { status: 'unavailable' });
    expect(feature.properties?.track).toBe(0);
  });

  it('does not rotate the ownship from a stale track', () => {
    let feature = ownshipFeature(position, { status: 'lastKnown', value: 45 });
    expect(feature.properties?.track).toBe(0);
  });
});

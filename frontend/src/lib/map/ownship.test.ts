import { describe, expect, it } from 'vitest';

import { ownshipFeature, positionCoordinates } from './ownship';

let position = { latitudeDegrees: 50.823, longitudeDegrees: 6.186 };

describe('ownship', () => {
  it('orders coordinates as longitude then latitude for GeoJSON', () => {
    expect(positionCoordinates(position)).toEqual([6.186, 50.823]);
  });

  it('carries the track into the feature properties', () => {
    expect(ownshipFeature(position, 270).properties).toEqual({ track: 270 });
  });

  it('falls back to zero when the track is unknown', () => {
    expect(ownshipFeature(position, null).properties).toEqual({ track: 0 });
  });
});

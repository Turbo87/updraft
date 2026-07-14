import { describe, expect, it } from 'vitest';
import type { PositionFix } from '$lib/protocol/generated/PositionFix';
import { ownshipFeature } from './ownship';

function positionFix(overrides: Partial<PositionFix> = {}): PositionFix {
  return {
    observedAtMs: 0,
    latitudeDegrees: 0,
    longitudeDegrees: 0,
    altitudeMeters: null,
    trackDegrees: null,
    groundSpeedMetersPerSecond: null,
    ...overrides,
  };
}

describe('ownshipFeature', () => {
  it('builds a point feature at the given position', () => {
    let feature = ownshipFeature(
      positionFix({ longitudeDegrees: 6.186, latitudeDegrees: 50.823, trackDegrees: 45 }),
    );
    expect(feature.geometry).toEqual({ type: 'Point', coordinates: [6.186, 50.823] });
    expect(feature.properties?.track).toBe(45);
  });

  it('defaults the track to zero when unset', () => {
    let feature = ownshipFeature(positionFix());
    expect(feature.properties?.track).toBe(0);
  });
});

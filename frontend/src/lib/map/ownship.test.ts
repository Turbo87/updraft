import { describe, expect, it } from 'vitest';
import { ownshipFeature } from './ownship';

describe('ownshipFeature', () => {
	it('builds a point feature at the given position', () => {
		const feature = ownshipFeature({ longitude: 6.186, latitude: 50.823, track: 45 });
		expect(feature.geometry).toEqual({ type: 'Point', coordinates: [6.186, 50.823] });
		expect(feature.properties?.track).toBe(45);
	});

	it('defaults the track to zero when unset', () => {
		const feature = ownshipFeature({ longitude: 0, latitude: 0 });
		expect(feature.properties?.track).toBe(0);
	});
});

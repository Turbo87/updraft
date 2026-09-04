import { expect, it } from 'vitest';

import { WaypointsStore } from './waypoints.svelte';

it('replaces waypoint status and ignores unrelated topics', () => {
  let store = new WaypointsStore();
  store.apply({ topic: 'airspace', value: { type: 'none' } });
  expect(store.initialized).toBe(false);
  let value = {
    generation: 2,
    sources: [
      {
        type: 'active' as const,
        sourceName: 'a.cup',
        waypointCount: 3,
        warnings: [{ line: 4, message: 'Skipped waypoint' }],
      },
    ],
  };
  store.apply({ topic: 'waypoints', value });
  expect(store.initialized).toBe(true);
  expect(store.current).toEqual(value);
  store.apply({ topic: 'waypoints', value: { generation: 3, sources: [] } });
  expect(store.current.sources).toEqual([]);
});

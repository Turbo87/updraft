import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import { MapState } from '$lib/map-state.svelte';
import { TrafficStore } from '$lib/stores/traffic.svelte';
import MapComponent from './Map.svelte';
import { waypointsFixture } from './waypoint.fixture';

it('renders waypoint types and removes the source when all files are removed', async () => {
  let mapState = new MapState();
  let component = await render(MapComponent, {
    mapState,
    traffic: new TrafficStore(),
    airspace: { type: 'none' },
    instruments: { gps: null, pressureAltitude: null, trueAirspeed: null, derived: null },
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    waypoints: {
      generation: 1,
      sources: [
        {
          type: 'active',
          sourceName: 'local.cup',
          waypointCount: 3,
          warnings: [],
        },
      ],
    },
    testMode: true,
    testWaypointData: waypointsFixture,
  });
  await vi.waitFor(() => {
    expect(mapState.map?.getLayer('waypoint-symbols')).toBeDefined();
    expect(mapState.map?.isSourceLoaded('waypoints')).toBe(true);
  });
  let map = mapState.map!;
  expect(
    new Set(map.querySourceFeatures('waypoints').map((feature) => feature.properties.kind)),
  ).toEqual(new Set([2, 3, 7]));
  expect(map.getLayer('waypoint-runways')).toBeDefined();
  await vi.waitFor(() => {
    let kinds = map
      .queryRenderedFeatures({ layers: ['waypoint-runways'] })
      .map((feature) => feature.properties.kind);
    expect(new Set(kinds)).toEqual(new Set([2, 3]));
  });
  expect(map.getLayoutProperty('waypoint-runways', 'icon-rotation-alignment')).toBe('map');
  await component.rerender({ waypoints: { generation: 2, sources: [] } });
  await vi.waitFor(() => expect(map.getSource('waypoints')).toBeUndefined());
});

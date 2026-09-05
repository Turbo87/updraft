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
  let icons = map.getLayoutProperty('waypoint-symbols', 'icon-image') as unknown[];
  for (let kind of [2, 4, 5]) {
    expect(icons[icons.indexOf(kind) + 1]).toBe('updraft-sdf:waypoint-airfield');
  }
  expect(map.hasImage('updraft-sdf:waypoint-airfield')).toBe(true);
  expect(map.getLayer('waypoint-runways')).toBeDefined();
  await vi.waitFor(() => {
    let kinds = map
      .queryRenderedFeatures({ layers: ['waypoint-runways'] })
      .map((feature) => feature.properties.kind);
    expect(new Set(kinds)).toEqual(new Set([2, 3]));
  });
  expect(map.getLayoutProperty('waypoint-runways', 'icon-rotation-alignment')).toBe('map');
  expect(map.getLayer('waypoint-labels')).toBeDefined();
  map.jumpTo({ zoom: 7 });
  await vi.waitFor(() => {
    let kinds = map
      .queryRenderedFeatures({ layers: ['waypoint-symbols'] })
      .map((feature) => feature.properties.kind);
    expect(new Set(kinds)).toEqual(new Set([2, 3]));
    expect(
      new Set(
        map
          .queryRenderedFeatures({ layers: ['waypoint-dots'] })
          .map((feature) => feature.properties.kind),
      ),
    ).toEqual(new Set([7]));
    expect(map.queryRenderedFeatures({ layers: ['waypoint-labels'] })).toEqual([]);
  });
  map.jumpTo({ zoom: 8 });
  await vi.waitFor(() => {
    expect(map.queryRenderedFeatures({ layers: ['waypoint-dots'] })).toEqual([]);
    expect(
      new Set(
        map
          .queryRenderedFeatures({ layers: ['waypoint-symbols'] })
          .map((feature) => feature.properties.kind),
      ),
    ).toEqual(new Set([2, 3, 7]));
  });
  map.jumpTo({ zoom: 11 });
  await vi.waitFor(() =>
    expect(map.queryRenderedFeatures({ layers: ['waypoint-labels'] }).length).toBeGreaterThan(0),
  );
  map.setLayoutProperty('waypoint-labels', 'text-variable-anchor', ['top']);
  let canvas = map.getCanvas();
  let denseWaypoints = {
    ...waypointsFixture,
    features: Array.from({ length: 10 }, (_, index) => ({
      ...waypointsFixture.features[0],
      id: `dense-${index}`,
      geometry: {
        type: 'Point' as const,
        coordinates: map
          .unproject([canvas.clientWidth / 2 + (index - 5) * 20, canvas.clientHeight / 2])
          .toArray(),
      },
      properties: { ...waypointsFixture.features[0].properties, name: String(index) },
    })),
  };
  await component.rerender({ testWaypointData: denseWaypoints });
  await vi.waitFor(() => {
    expect(map.queryRenderedFeatures({ layers: ['waypoint-labels'] }).length).toBeGreaterThan(0);
    for (let label of map.queryRenderedFeatures({ layers: ['waypoint-labels'] })) {
      expect(label.properties.name).toMatch(/^\d$/);
    }
  });
  let paddedLabelCount = map.queryRenderedFeatures({ layers: ['waypoint-labels'] }).length;
  map.setLayoutProperty('waypoint-labels', 'text-padding', 2);
  await vi.waitFor(() => {
    expect(map.queryRenderedFeatures({ layers: ['waypoint-labels'] }).length).toBeGreaterThan(
      paddedLabelCount,
    );
  });
  await component.rerender({
    testWaypointData: {
      ...waypointsFixture,
      features: waypointsFixture.features.filter((feature) => feature.properties?.kind === 7),
    },
  });
  map.jumpTo({ zoom: 8 });
  await vi.waitFor(() => {
    let labels = map.queryRenderedFeatures({ layers: ['waypoint-labels'] });
    expect(new Set(labels.map((feature) => feature.properties.kind))).toEqual(new Set([7]));
  });
  await component.rerender({ waypoints: { generation: 2, sources: [] } });
  await vi.waitFor(() => expect(map.getSource('waypoints')).toBeUndefined());
});

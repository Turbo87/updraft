import type { StyleLayer } from 'maplibre-gl';
import type { UpdraftClient } from '$lib/client';

import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import { FakeClient } from '$lib/client/fake';
import { MapState } from '$lib/map-state.svelte';
import { TrafficStore } from '$lib/stores/traffic.svelte';
import { arrivalFixture } from './arrival.fixture';
import MapComponent from './Map.svelte';

function evaluate(layer: StyleLayer, type: 'layout' | 'paint', name: string, properties: object) {
  let values = layer[type] as {
    get(name: string): { evaluate(feature: object, state: object): { toString(): string } };
  };
  return values.get(name).evaluate({ type: 1, properties }, {}).toString();
}

it('renders arrival labels, colors and catalog filters while retaining waypoint selection', async () => {
  let client = new FakeClient();
  let subscribe = vi.spyOn(client as UpdraftClient, 'subscribeArrivals');
  let mapState = new MapState();
  let units = {
    altitude: 'm' as const,
    distance: 'km' as const,
    speed: 'km/h' as const,
    verticalSpeed: 'm/s' as const,
  };
  let component = await render(MapComponent, {
    client,
    mapState,
    units,
    traffic: new TrafficStore(),
    airspace: { type: 'none' },
    instruments: { gps: null, pressureAltitude: null, trueAirspeed: null, derived: null },
    waypoints: {
      generation: 1,
      sources: [{ type: 'active', sourceName: 'fields.cup', waypointCount: 6, warnings: [] }],
    },
    testMode: true,
    testWaypointData: { type: 'FeatureCollection', features: [] },
  });
  component.container.style.height = '300px';
  await vi.waitFor(() =>
    expect(mapState.map?.hasImage('updraft-sdf:waypoint-airfield')).toBe(true),
  );
  let map = mapState.map!;
  map.resize();
  await map.once('idle');
  await component.rerender({ testWaypointData: arrivalFixture });
  await map.once('idle');
  expect(map.hasImage('updraft-sdf:waypoint-airfield')).toBe(true);
  expect(map.queryRenderedFeatures({ layers: ['waypoint-symbols'] })).toHaveLength(6);
  let foreign = {
    ...arrivalFixture.features[0],
    id: '2:0:0',
    properties: { ...arrivalFixture.features[0].properties, catalogGeneration: 2 },
  };
  let resource = { ...arrivalFixture, features: [...arrivalFixture.features, foreign] };
  let url = URL.createObjectURL(new Blob([JSON.stringify(resource)]));
  try {
    client.emitArrivals({ generation: 1, url });
    await map.once('idle');
    await vi.waitFor(() => {
      expect(map.getLayer('arrival-symbols')).toBeDefined();
      expect(map.queryRenderedFeatures({ layers: ['arrival-symbols'] })).toHaveLength(6);
      expect(
        map.queryRenderedFeatures({ layers: ['waypoint-symbols', 'waypoint-labels'] }),
      ).toEqual([]);
    });
    expect(map.queryRenderedFeatures({ layers: ['waypoint-hit'] })).toHaveLength(6);
    let labels = map.getLayer('arrival-labels')!;
    let symbols = map.getLayer('arrival-symbols')!;
    let runways = map.getLayer('arrival-runways')!;
    let rows = arrivalFixture.features.map(({ properties }) => ({
      label: evaluate(labels, 'layout', 'text-field', properties!),
      color: evaluate(symbols, 'paint', 'icon-color', properties!),
      runway: evaluate(runways, 'paint', 'icon-halo-color', properties!),
    }));
    expect(rows).toMatchInlineSnapshot(`
      [
        {
          "color": "rgba(0,130,54,1)",
          "label": "Reachable
      +250m",
          "runway": "rgba(0,130,54,1)",
        },
        {
          "color": "rgba(254,154,0,1)",
          "label": "Below reserve
      -100m",
          "runway": "rgba(254,154,0,1)",
        },
        {
          "color": "rgba(193,0,7,1)",
          "label": "Unreachable
      -200m",
          "runway": "rgba(193,0,7,1)",
        },
        {
          "color": "rgba(0,130,54,1)",
          "label": "Stale
      (+250m)",
          "runway": "rgba(0,130,54,1)",
        },
        {
          "color": "rgba(112,8,231,1)",
          "label": "Unavailable",
          "runway": "rgba(112,8,231,1)",
        },
        {
          "color": "rgba(0,130,54,1)",
          "label": "At reserve
      +0m",
          "runway": "rgba(0,130,54,1)",
        },
      ]
    `);
    await component.rerender({ units: { ...units, altitude: 'ft' } });
    expect(subscribe).toHaveBeenCalledTimes(1);
    await vi.waitFor(() =>
      expect(
        evaluate(
          map.getLayer('arrival-labels')!,
          'layout',
          'text-field',
          arrivalFixture.features[3].properties!,
        ),
      ).toBe('Stale\n(+820ft)'),
    );
    expect(
      evaluate(map.getLayer('arrival-labels')!, 'layout', 'text-field', {
        name: 'Just below',
        arrivalMarginMeters: -0.1,
      }),
    ).toBe('Just below\n+0ft');
    let error = new Error('Arrival worker stopped');
    let log = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      subscribe.mock.calls[0][2](error);
      map.fire('sourcedata', { sourceId: 'arrivals', sourceDataType: 'content' });
      await vi.waitFor(() => {
        expect(map.getSource('arrivals')).toBeUndefined();
        expect(map.queryRenderedFeatures({ layers: ['waypoint-symbols'] })).toHaveLength(6);
      });
      expect(log).toHaveBeenCalledExactlyOnceWith('Arrival subscription failed', error);
    } finally {
      log.mockRestore();
    }
  } finally {
    await component.unmount();
    URL.revokeObjectURL(url);
  }
});

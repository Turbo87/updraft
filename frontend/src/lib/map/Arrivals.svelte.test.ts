import type { GeoJSONSource } from 'maplibre-gl';
import type { UpdraftClient } from '$lib/client';

import { tick } from 'svelte';
import { LngLatBounds } from 'maplibre-gl';
import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import { FakeClient } from '$lib/client/fake';
import { MapState } from '$lib/map-state.svelte';
import { TrafficStore } from '$lib/stores/traffic.svelte';
import MapComponent from './Map.svelte';
import { waypointsFixture } from './waypoint.fixture';

it('updates viewport arrivals and closes subscriptions on catalog changes and unmount', async () => {
  let client = new FakeClient();
  let subscribe = vi.spyOn(client as UpdraftClient, 'subscribeArrivals');
  let mapState = new MapState();
  let waypoints = {
    generation: 1,
    sources: [{ type: 'active' as const, sourceName: 'local.cup', waypointCount: 3, warnings: [] }],
  };
  let component = await render(MapComponent, {
    hillshadeDirection: 'fixed',
    client,
    mapState,
    waypoints,
    traffic: new TrafficStore(),
    airspace: { generation: 0, sources: [] },
    instruments: {
      gps: null,
      pressureAltitude: null,
      trueAirspeed: null,
      terrainElevation: null,
      altitudeAgl: null,
      solarPosition: null,
      derived: null,
    },
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    testMode: true,
    testWaypointData: { type: 'FeatureCollection', features: [] },
  });
  await vi.waitFor(() => expect(subscribe).toHaveBeenCalledTimes(1));
  let map = mapState.map!;
  expect(subscribe.mock.calls[0][0]).toEqual(map.getBounds().toArray().flat());
  let subscription = subscribe.mock.results[0].value;
  let move = vi.spyOn(subscription, 'updateViewport');
  let close = vi.spyOn(subscription, 'close');
  let url = URL.createObjectURL(new Blob([JSON.stringify(waypointsFixture)]));
  try {
    client.emitArrivals({ generation: 0, url });
    await tick();
    expect(map.getSource('arrivals')).toBeUndefined();
    client.emitArrivals({ generation: 1, url });
    await vi.waitFor(() => expect(map.getSource('arrivals')).toBeDefined());
    let source = map.getSource('arrivals') as GeoJSONSource;
    expect(await source.getData()).toEqual(waypointsFixture);
    await vi.waitFor(() => expect(map.getLayer('arrival-symbols')).toBeDefined());
    expect(map.getLayer('arrival-labels')).toBeDefined();
    let invalidUrl = URL.createObjectURL(new Blob(['invalid GeoJSON']));
    let loadError = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      client.emitArrivals({ generation: 1, url: invalidUrl });
      await vi.waitFor(() =>
        expect(loadError).toHaveBeenCalledExactlyOnceWith(
          'Failed to load arrival resource',
          expect.any(Error),
        ),
      );
    } finally {
      loadError.mockRestore();
      URL.revokeObjectURL(invalidUrl);
    }
    map.jumpTo({ center: [179, 50], zoom: 5 });
    expect(move).toHaveBeenLastCalledWith(map.getBounds().toArray().flat());
    expect(map.getSource('arrivals')).toBe(source);
    move.mockClear();
    let bounds = vi.spyOn(map, 'getBounds');
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    try {
      bounds.mockReturnValue(new LngLatBounds([6, 50], [7, 51]));
      map.fire('move');
      vi.advanceTimersByTime(50);
      bounds.mockReturnValue(new LngLatBounds([7, 51], [8, 52]));
      map.fire('move');
      vi.advanceTimersByTime(49);
      expect(move).not.toHaveBeenCalled();
      vi.advanceTimersByTime(1);
      expect(move).toHaveBeenCalledExactlyOnceWith([7, 51, 8, 52]);

      move.mockClear();
      bounds.mockReturnValue(new LngLatBounds([8, 52], [9, 53]));
      map.fire('move');
      vi.advanceTimersByTime(25);
      map.fire('moveend');
      expect(move).toHaveBeenCalledExactlyOnceWith([8, 52, 9, 53]);
      vi.advanceTimersByTime(100);
      map.fire('moveend');
      expect(move).toHaveBeenCalledTimes(1);

      move.mockClear();
      map.fire('move');
      bounds.mockRestore();
      await component.rerender({ waypoints: { ...waypoints, generation: 2 } });
      vi.advanceTimersByTime(100);
      expect(move).not.toHaveBeenCalled();
    } finally {
      bounds.mockRestore();
      vi.useRealTimers();
    }
    await vi.waitFor(() => expect(subscribe).toHaveBeenCalledTimes(2));
    expect(close).toHaveBeenCalledTimes(1);
    expect(map.getSource('arrivals')).toBeUndefined();
    client.emitArrivals({ generation: 2, url });
    await vi.waitFor(() => expect(map.getSource('arrivals')).toBeDefined());
    let error = new Error('Arrival worker stopped');
    let log = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      subscribe.mock.calls[1][2](error);
      await vi.waitFor(() => expect(map.getSource('arrivals')).toBeUndefined());
      expect(log).toHaveBeenCalledExactlyOnceWith('Arrival subscription failed', error);
    } finally {
      log.mockRestore();
    }
    let removedClose = vi.spyOn(subscribe.mock.results[1].value, 'close');
    await component.rerender({ waypoints: { generation: 3, sources: [] } });
    expect(removedClose).toHaveBeenCalledTimes(1);
    await component.rerender({ waypoints: { ...waypoints, generation: 4 } });
    await vi.waitFor(() => expect(subscribe).toHaveBeenCalledTimes(3));
    let finalClose = vi.spyOn(subscribe.mock.results[2].value, 'close');
    await component.unmount();
    expect(finalClose).toHaveBeenCalledTimes(1);
  } finally {
    URL.revokeObjectURL(url);
  }
});

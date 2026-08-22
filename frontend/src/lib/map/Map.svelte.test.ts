import type { Map as MapLibreMap } from 'maplibre-gl';
import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';

import { afterEach, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import { MapState } from '$lib/map-state.svelte';
import { TrafficStore } from '$lib/stores/traffic.svelte';
import { AIRSPACE_BROWSER_FIXTURE } from './airspace.fixture';
import MapComponent from './Map.svelte';

const instruments = {
  gps: null,
  pressureAltitude: null,
  trueAirspeed: null,
  derived: null,
};

const positionInstruments = {
  gps: {
    position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
    altitudeMeters: 410,
    groundSpeedMetersPerSecond: 31,
    trackDegrees: 90,
    fixTime: null,
    stale: true,
  },
  pressureAltitude: null,
  trueAirspeed: null,
  derived: null,
};

const units = {
  altitude: 'm' as const,
  distance: 'km' as const,
  speed: 'km/h' as const,
  verticalSpeed: 'm/s' as const,
};

afterEach(() => {
  vi.restoreAllMocks();
});

async function renderMap(
  airspace: AirspaceStatus,
  traffic = new TrafficStore(),
): Promise<MapLibreMap> {
  let mapState = new MapState();
  await render(MapComponent, {
    instruments,
    mapState,
    traffic,
    units,
    airspace,
    testMode: true,
    testAirspaceData: AIRSPACE_BROWSER_FIXTURE,
  });

  await vi.waitFor(() => {
    expect(mapState.map).toBeDefined();
  });
  return mapState.map!;
}

function airspaceStyle(map: MapLibreMap) {
  let style = map.getStyle();
  let source = style.sources.airspace;
  if (!source || source.type !== 'geojson') throw new Error('Airspace source is not GeoJSON');

  let layers = style.layers
    .filter(({ id }) => id === 'airspace-fill' || id === 'airspace-outline')
    .map((layer) => ({
      id: layer.id,
      type: layer.type,
      source: 'source' in layer ? layer.source : undefined,
      paint: 'paint' in layer ? layer.paint : undefined,
    }));

  return {
    source: { type: source.type, maxzoom: source.maxzoom },
    layers,
  };
}

it.each([
  { type: 'none' } as const,
  { type: 'unavailable', sourceName: 'broken.txt', error: 'parseFailed' } as const,
])('does not add airspace map data for the $type state', async (airspace) => {
  let map = await renderMap(airspace);

  expect(map.getSource('airspace')).toBeUndefined();
  expect(map.getLayer('airspace-hit')).toBeUndefined();
  expect(map.getLayer('airspace-fill')).toBeUndefined();
  expect(map.getLayer('airspace-outline')).toBeUndefined();
});

it('adds the airspace source and both layers for the active state', async () => {
  let consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
  let map = await renderMap({
    type: 'active',
    sourceName: 'rheinland.txt',
    airspaceCount: 2,
    generation: 3,
  });

  await vi.waitFor(() => {
    expect(map.getSource('airspace')).toBeDefined();
    expect(map.getLayer('airspace-hit')).toBeDefined();
    expect(map.getLayer('airspace-fill')).toBeDefined();
    expect(map.getLayer('airspace-outline')).toBeDefined();
  });
  expect(airspaceStyle(map)).toMatchSnapshot();
  expect(consoleError).not.toHaveBeenCalled();
});

it('queries airspace through the transparent hit layer', async () => {
  let map = await renderMap({
    type: 'active',
    sourceName: 'rheinland.txt',
    airspaceCount: 2,
    generation: 3,
  });

  await vi.waitFor(() => {
    expect(map.getLayer('airspace-hit')).toBeDefined();
  });
  await vi.waitFor(() => {
    let features = map.queryRenderedFeatures(map.project([6.175, 50.82]), {
      layers: ['airspace-hit'],
    });
    let overlappingFeatures = map.queryRenderedFeatures(map.project([6.182, 50.82]), {
      layers: ['airspace-hit'],
    });

    expect(features.map(({ id }) => id)).toEqual([0]);
    expect(overlappingFeatures.map(({ id }) => id)).toEqual([1, 0]);
  });
  expect(map.getPaintProperty('airspace-hit', 'fill-opacity')).toBe(0);
});

it('queries traffic within the transparent 24 pixel hit radius', async () => {
  let traffic = new TrafficStore();
  let position = { latitudeDegrees: 50.823, longitudeDegrees: 6.186 };
  traffic.apply({
    topic: 'traffic',
    value: {
      type: 'snapshot',
      value: [
        {
          id: 'flarm:000001',
          position,
          altitudeMslMeters: 500,
          trafficType: 'glider',
          trackDegrees: 90,
          alarmLevel: 'none',
          stale: false,
        },
      ],
    },
  });
  let map = await renderMap({ type: 'none' }, traffic);

  await vi.waitFor(() => {
    expect(map.getLayer('traffic-hit')).toBeDefined();
  });

  let targetPoint = map.project([position.longitudeDegrees, position.latitudeDegrees]);
  await vi.waitFor(() => {
    let features = map.queryRenderedFeatures(targetPoint, { layers: ['traffic-hit'] });
    expect(features.map(({ id }) => id)).toEqual(['flarm:000001']);
  });

  let inside = map.queryRenderedFeatures([targetPoint.x + 23, targetPoint.y], {
    layers: ['traffic-hit'],
  });
  let outside = map.queryRenderedFeatures([targetPoint.x + 25, targetPoint.y], {
    layers: ['traffic-hit'],
  });

  expect(map.getPaintProperty('traffic-hit', 'circle-radius')).toBe(24);
  expect(map.getPaintProperty('traffic-hit', 'circle-opacity')).toBe(0);
  expect(inside.map(({ id }) => id)).toEqual(['flarm:000001']);
  expect(outside).toEqual([]);

  await userEvent.keyboard('d');
  let checkbox = page.getByRole('checkbox', { name: 'Traffic hit areas' });
  await checkbox.click();
  await vi.waitFor(() => {
    expect(map.getPaintProperty('traffic-hit', 'circle-opacity')).toBe(0.2);
  });

  let visible = map.queryRenderedFeatures([targetPoint.x + 23, targetPoint.y], {
    layers: ['traffic-hit'],
  });
  expect(map.getPaintProperty('traffic-hit', 'circle-radius')).toBe(24);
  expect(visible.map(({ id }) => id)).toEqual(inside.map(({ id }) => id));
});

it('publishes the map and camera values through the shared map state', async () => {
  let mapState = new MapState();
  await render(MapComponent, {
    instruments,
    traffic: new TrafficStore(),
    units,
    airspace: { type: 'none' },
    mapState,
    testMode: true,
  });
  await vi.waitFor(() => {
    expect(mapState.map).toBeDefined();
  });
  let map = mapState.map!;

  map.jumpTo({ center: [7, 51], zoom: 9, bearing: 15, pitch: 20 });

  await vi.waitFor(() => {
    expect(mapState.center).toEqual({ lng: 7, lat: 51 });
    expect(mapState.zoom).toBe(9);
    expect(mapState.bearing).toBe(15);
    expect(mapState.pitch).toBe(20);
  });
});

it('does not show the built-in attribution control', async () => {
  await renderMap({ type: 'none' });

  expect(document.querySelector('.maplibregl-ctrl-attrib')).toBeNull();
});

it('returns to follow mode without a position and follows the next position', async () => {
  let mapState = new MapState();
  let traffic = new TrafficStore();
  let view = await render(MapComponent, {
    instruments,
    mapState,
    traffic,
    units,
    airspace: { type: 'none' },
    testMode: true,
  });
  await vi.waitFor(() => {
    expect(mapState.map).toBeDefined();
  });
  let map = mapState.map!;
  await vi.waitFor(() => {
    expect(map.isStyleLoaded()).toBe(true);
    expect(map.hasImage('updraft-sdf:glider')).toBe(true);
  });
  let initialCenter = map.getCenter().toArray();

  map.fire('dragstart');
  expect(mapState.followMode).toBe(false);
  let returnButton = page.getByRole('button', { name: 'Return to position' });
  await expect.element(returnButton).toBeVisible();

  let button = document.querySelector<HTMLButtonElement>('button[aria-label="Return to position"]');
  if (!button) throw new Error('Return to position button is not available');
  button.click();
  expect(mapState.followMode).toBe(true);
  await expect.element(returnButton).not.toBeInTheDocument();
  expect(map.getCenter().toArray()).toEqual(initialCenter);

  map.jumpTo({ zoom: 9, bearing: 15, pitch: 20 });
  await view.rerender({
    instruments: positionInstruments,
    mapState,
    traffic,
    units,
    airspace: { type: 'none' },
    testMode: true,
  });
  await vi.waitFor(() => {
    expect(map.getCenter().toArray()).toEqual([
      expect.closeTo(positionInstruments.gps.position.longitudeDegrees, 6),
      expect.closeTo(positionInstruments.gps.position.latitudeDegrees, 6),
    ]);
    expect(map.getZoom()).toBe(9);
    expect(map.getBearing()).toBe(15);
    expect(map.getPitch()).toBe(20);
  });

  await view.rerender({
    instruments,
    mapState,
    traffic,
    units,
    airspace: { type: 'none' },
    testMode: true,
  });
  await vi.waitFor(() => {
    expect(map.getLayer('ownship-symbol')).toBeUndefined();
    expect(map.getSource('ownship')).toBeUndefined();
  });
});

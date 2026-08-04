import type { Map as MapLibreMap } from 'maplibre-gl';
import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';

import { afterEach, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import { MapState } from '$lib/map-state.svelte';
import { TrafficStore } from '$lib/stores/traffic.svelte';
import { AIRSPACE_BROWSER_FIXTURE } from './airspace.fixture';
import MapComponent from './Map.svelte';

const instruments = {
  gps: null,
  pressureAltitude: null,
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

async function renderMap(airspace: AirspaceStatus): Promise<MapLibreMap> {
  let mapState = new MapState();
  await render(MapComponent, {
    instruments,
    mapState,
    traffic: new TrafficStore(),
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

it.each([
  { type: 'none' } as const,
  { type: 'unavailable', sourceName: 'broken.txt', error: 'parseFailed' } as const,
])('does not add airspace map data for the $type state', async (airspace) => {
  let map = await renderMap(airspace);

  expect(map.getSource('airspace')).toBeUndefined();
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
    expect(map.getLayer('airspace-fill')).toBeDefined();
    expect(map.getLayer('airspace-outline')).toBeDefined();
  });
  expect(consoleError).not.toHaveBeenCalled();
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

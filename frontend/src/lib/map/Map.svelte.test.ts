import type { Map as MapLibreMap } from 'maplibre-gl';
import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';

import { afterEach, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import { TrafficStore } from '$lib/stores/traffic.svelte';
import { AIRSPACE_BROWSER_FIXTURE } from './airspace.fixture';
import MapComponent from './Map.svelte';

type TestWindow = Window & {
  __updraftTest?: { map: MapLibreMap };
};

const instruments = {
  position: null,
  altitudeMslMeters: null,
  trackDegrees: null,
  groundSpeedMetersPerSecond: null,
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
  await render(MapComponent, {
    instruments,
    traffic: new TrafficStore(),
    units,
    airspace,
    testMode: true,
    testAirspaceData: AIRSPACE_BROWSER_FIXTURE,
  });

  await vi.waitFor(() => {
    expect((window as TestWindow).__updraftTest?.map).toBeDefined();
  });
  return (window as TestWindow).__updraftTest!.map;
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

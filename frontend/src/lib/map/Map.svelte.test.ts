import type { GeoJSONSourceSpecification, Map as MapLibreMap } from 'maplibre-gl';
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
  testAirspaceData: GeoJSONSourceSpecification['data'] = AIRSPACE_BROWSER_FIXTURE,
): Promise<MapLibreMap> {
  let mapState = new MapState();
  await render(MapComponent, {
    instruments,
    mapState,
    traffic,
    units,
    airspace,
    testMode: true,
    testAirspaceData,
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
    .filter(
      (layer) => 'source' in layer && layer.source === 'airspace' && layer.id !== 'airspace-hit',
    )
    .map((layer) => ({
      id: layer.id,
      type: layer.type,
      source: 'source' in layer ? layer.source : undefined,
      paint: 'paint' in layer ? layer.paint : undefined,
      filter: 'filter' in layer ? layer.filter : undefined,
      layout: layer.layout,
    }));

  return {
    source: { type: source.type, maxzoom: source.maxzoom },
    layers,
  };
}

it.each([
  { generation: 0, sources: [] } satisfies AirspaceStatus,
  {
    generation: 0,
    sources: [{ type: 'unavailable', sourceName: 'broken.txt', error: 'parseFailed' }],
  } satisfies AirspaceStatus,
])('does not add airspace map data for the $type state', async (airspace) => {
  let map = await renderMap(airspace);

  expect(map.getSource('airspace')).toBeUndefined();
  expect(map.getLayer('airspace-hit')).toBeUndefined();
  expect(map.getLayer('airspace-inner-band')).toBeUndefined();
  expect(map.getLayer('airspace-outline')).toBeUndefined();
});

it('adds the airspace source and styled layers for the active state', async () => {
  let consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
  let map = await renderMap({
    generation: 3,
    sources: [{ type: 'active', sourceName: 'rheinland.txt', airspaceCount: 2 }],
  });

  await vi.waitFor(() => {
    expect(map.getSource('airspace')).toBeDefined();
    expect(map.getLayer('airspace-hit')).toBeDefined();
    expect(map.getLayer('airspace-inner-band')).toBeDefined();
    expect(map.getLayer('airspace-outline')).toBeDefined();
  });
  expect(
    map
      .getStyle()
      .layers.filter((layer) => 'source' in layer && layer.source === 'airspace')
      .map(({ id }) => id),
  ).toEqual(['airspace-hit', 'airspace-inner-band', 'airspace-outline']);
  expect(airspaceStyle(map)).toMatchSnapshot();
  expect(consoleError).not.toHaveBeenCalled();
});

it('queries airspace through the transparent hit layer', async () => {
  let map = await renderMap({
    generation: 3,
    sources: [{ type: 'active', sourceName: 'rheinland.txt', airspaceCount: 2 }],
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

    expect(features.map(({ id }) => id)).toEqual(['1:0:0']);
    expect(overlappingFeatures.map(({ id }) => id)).toEqual(['1:0:1', '1:0:0']);
  });
  expect(map.getPaintProperty('airspace-hit', 'fill-opacity')).toBe(0);
});

it.each([5, 6, 6.5, 7, 7.5, 8, 9])(
  'scales inner airspace bands at zoom %s without filling interiors',
  async (zoom) => {
    let map = await renderMap(
      {
        generation: 3,
        sources: [{ type: 'active', sourceName: 'rheinland.txt', airspaceCount: 2 }],
      },
      new TrafficStore(),
      {
        ...AIRSPACE_BROWSER_FIXTURE,
        features: AIRSPACE_BROWSER_FIXTURE.features.map((feature) =>
          feature.id === '1:0:1'
            ? { ...feature, properties: { ...feature.properties, type: 21 } }
            : feature,
        ),
      },
    );
    map.jumpTo({ zoom });

    await vi.waitFor(() => {
      let bands = map
        .queryRenderedFeatures()
        .filter(({ layer }) => layer.id === 'airspace-inner-band');
      expect(bands).toHaveLength(2);
      for (let { id, layer } of bands) {
        let fullWidth = id === '1:0:1' ? 10 : 7;
        let width = Math.max(0, Math.min(fullWidth, ((zoom - 6) / 2) * fullWidth));
        if (layer.type !== 'line') throw new Error('Airspace band is not a line');
        expect(layer.paint?.['line-width']).toBe(width);
        expect(layer.paint?.['line-offset']).toBe(width / 2);
        expect(layer.paint?.['line-opacity']).toBe(id === '1:0:1' ? 0.25 : 0.2);
      }
      expect(
        map
          .getStyle()
          .layers.filter((layer) => layer.type === 'fill' && layer.source === 'airspace')
          .map(({ id }) => id),
      ).toEqual(['airspace-hit']);
      expect(
        map
          .queryRenderedFeatures(map.project([6.182, 50.82]), { layers: ['airspace-hit'] })
          .map(({ id }) => id),
      ).toEqual(['1:0:1', '1:0:0']);
    });
  },
);

it.each([
  { type: 0, icaoClass: 0, color: [20, 71, 230], bands: ['airspace-inner-band'] },
  { type: 0, icaoClass: 1, color: [20, 71, 230], bands: ['airspace-inner-band'] },
  { type: 0, icaoClass: 2, color: [20, 71, 230], bands: ['airspace-inner-band'] },
  { type: 0, icaoClass: 3, color: [20, 71, 230], bands: ['airspace-inner-band'] },
  { type: 0, icaoClass: 4, color: [20, 71, 230], bands: [] },
  ...[1, 2, 3, 29].map((type) => ({
    type,
    icaoClass: 8,
    color: [193, 0, 7],
    bands: ['airspace-inner-band'],
    dash: [4, 3],
  })),
  { type: 4, icaoClass: 3, color: [20, 71, 230], bands: ['airspace-inner-band'], dash: [4, 3] },
  { type: 5, icaoClass: 8, color: [49, 65, 88], bands: [], dash: [4, 3, 1, 3] },
  ...[6, 13, 23, 24].map((type) => ({
    type,
    icaoClass: 8,
    color: [20, 71, 230],
    bands: ['airspace-inner-band'],
    dash: [3, 3],
  })),
  ...[10, 11].map((type) => ({
    type,
    icaoClass: 8,
    color: [0, 130, 54],
    bands: [],
    width: 1.5,
  })),
  { type: 33, icaoClass: 8, color: [0, 130, 54], bands: [], width: 1.5, dash: [0, 3] },
  { type: 19, icaoClass: 8, color: [0, 130, 54], bands: ['airspace-inner-band'] },
  { type: 21, icaoClass: 8, color: [208, 135, 0], bands: ['airspace-inner-band'], opacity: 0.8 },
  ...[3, 8].map((icaoClass) => ({
    type: 25,
    icaoClass,
    color: [69, 85, 108],
    bands: ['airspace-inner-band'],
    dash: [0, 3],
  })),
  ...[1, 19].map((type) => ({
    type,
    icaoClass: 2,
    color: type === 1 ? [193, 0, 7] : [0, 130, 54],
    bands: ['airspace-inner-band'],
    ...(type === 1 && { dash: [4, 3] }),
  })),
  { type: 99, icaoClass: 8, color: [20, 71, 230], bands: [] },
])('renders airspace boundaries for type $type and class $icaoClass', async (scenario) => {
  let { type, icaoClass, color, bands } = scenario;
  let fixture = AIRSPACE_BROWSER_FIXTURE.features[0];
  let map = await renderMap(
    { generation: 1, sources: [{ type: 'active', sourceName: 'airspace.txt', airspaceCount: 1 }] },
    new TrafficStore(),
    {
      type: 'FeatureCollection',
      features: [{ ...fixture, properties: { ...fixture.properties, type, icaoClass } }],
    },
  );

  await vi.waitFor(() => {
    let features = map.queryRenderedFeatures().filter(({ source }) => source === 'airspace');
    let lines = features.filter(({ layer }) => layer.type === 'line');
    let boundaries = lines.filter(({ layer }) => !layer.id.endsWith('-band'));
    expect(boundaries).toHaveLength(1);
    let boundary = boundaries[0].layer;
    if (boundary.type !== 'line') throw new Error('Airspace boundary is not a line');
    expect(boundary.paint?.['line-color']).toMatchObject({
      r: color[0] / 255,
      g: color[1] / 255,
      b: color[2] / 255,
    });
    let dash = 'dash' in scenario ? scenario.dash : [1, 0];
    expect(boundary.paint?.['line-dasharray']).toEqual({ from: dash, to: dash });
    expect(boundary.layout?.['line-cap']).toBe(type === 33 || type === 25 ? 'round' : 'butt');
    expect(boundary.paint?.['line-width']).toBe('width' in scenario ? scenario.width : 2);
    expect(boundary.paint?.['line-opacity']).toBe('opacity' in scenario ? scenario.opacity : 1);
    expect(
      lines.filter(({ layer }) => layer.id.endsWith('-band')).map(({ layer }) => layer.id),
    ).toEqual(bands);
    if (type === 25) {
      let band = lines.find(({ layer }) => layer.id === 'airspace-inner-band')?.layer;
      if (band?.type !== 'line') throw new Error('Military airspace band is not available');
      expect(band.paint?.['line-color']).toMatchObject({ r: 69 / 255, g: 85 / 255, b: 108 / 255 });
      expect(band.paint?.['line-opacity']).toBe(0.2);
    }
    expect(
      map
        .queryRenderedFeatures(map.project([6.175, 50.82]), { layers: ['airspace-hit'] })
        .map(({ id }) => id),
    ).toEqual(['1:0:0']);
  });
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
  let map = await renderMap({ generation: 0, sources: [] }, traffic);

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
  let checkbox = page.getByRole('checkbox', { name: 'Traffic and waypoint hit areas' });
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
    airspace: { generation: 0, sources: [] },
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
  await renderMap({ generation: 0, sources: [] });

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
    airspace: { generation: 0, sources: [] },
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
    airspace: { generation: 0, sources: [] },
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
    airspace: { generation: 0, sources: [] },
    testMode: true,
  });
  await vi.waitFor(() => {
    expect(map.getLayer('ownship-symbol')).toBeUndefined();
    expect(map.getSource('ownship')).toBeUndefined();
  });
});

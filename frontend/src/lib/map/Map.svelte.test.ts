import type * as GeoJSON from 'geojson';
import type { GeoJSONSource, GeoJSONSourceSpecification, Map as MapLibreMap } from 'maplibre-gl';
import type { AirspaceStatus } from '#lib/protocol/generated/AirspaceStatus.js';
import type { TrailColorMode } from '#lib/protocol/generated/TrailColorMode.js';

import { afterEach, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import { FakeClient } from '#lib/client/fake.js';
import { instrumentsFixture } from '#lib/instruments.fixture.js';
import { MapState } from '#lib/map-state.svelte.js';
import { settingsFixture } from '#lib/settings.fixture.js';
import { EMPTY_DERIVED_INSTRUMENTS } from '#lib/stores/instruments.svelte.js';
import { SettingsStore } from '#lib/stores/settings.svelte.js';
import { TrafficStore } from '#lib/stores/traffic.svelte.js';
import { TrailStore } from '#lib/stores/trail.svelte.js';
import { trafficTarget } from '#lib/traffic.fixture.js';
import { AIRSPACE_BROWSER_FIXTURE } from './airspace.fixture';
import { deadReckon, groundMotion } from './dead-reckoning';
import { mapProps } from './map.fixture';
import MapComponent from './Map.svelte';
import { trailColor, trailTranslate } from './trail';

const instruments = instrumentsFixture();

const positionInstruments = {
  ...instrumentsFixture(),
  gps: {
    position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
    altitudeMeters: 410,
    groundSpeedMetersPerSecond: 31,
    trackDegrees: 90,
    turnRateDegreesPerSecond: null,
    fixTime: null,
    stale: true,
  },
};

const movingInstruments = {
  ...positionInstruments,
  gps: {
    ...positionInstruments.gps,
    groundSpeedMetersPerSecond: 30,
    trackDegrees: 90,
    turnRateDegreesPerSecond: 20,
    stale: false,
  },
};

const units = {
  altitude: 'm' as const,
  distance: 'km' as const,
  speed: 'km/h' as const,
  verticalSpeed: 'm/s' as const,
};

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

/**
 * Advances animation frames and `performance.now()` only through the test.
 *
 * Two real frames first run a frame that the map requested before.
 */
async function useFakeFrames() {
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  vi.useFakeTimers({ toFake: ['requestAnimationFrame', 'cancelAnimationFrame', 'performance'] });
}

/** The center of the dead-reckoned ownship after `seconds`, as longitude and latitude. */
function reckonedCenter(seconds: number): [number, number] {
  let { gps } = movingInstruments;
  let motion = groundMotion(gps);
  if (!motion) throw new Error('The moving fixture has no ground motion');
  let { position } = deadReckon(gps.position, motion, seconds);
  return [position.longitudeDegrees, position.latitudeDegrees];
}

function expectCenter(map: MapLibreMap, center: [number, number]) {
  let [longitude, latitude] = map.getCenter().toArray();
  expect(longitude).toBeCloseTo(center[0], 9);
  expect(latitude).toBeCloseTo(center[1], 9);
}

async function renderLoadedMap(instruments = instrumentsFixture()) {
  let mapState = new MapState();
  let view = await render(MapComponent, { ...mapProps(mapState), instruments, units });
  await vi.waitFor(() => {
    expect(mapState.map?.loaded()).toBe(true);
    expect(mapState.map?.hasImage('updraft-sdf:glider')).toBe(true);
  });
  return { map: mapState.map!, mapState, view };
}

async function renderMap(
  airspace: AirspaceStatus,
  traffic = new TrafficStore(),
  testAirspaceData: GeoJSONSourceSpecification['data'] = AIRSPACE_BROWSER_FIXTURE,
): Promise<MapLibreMap> {
  let mapState = new MapState();
  await render(MapComponent, {
    ...mapProps(mapState),
    instruments,
    traffic,
    units,
    airspace,
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
        trafficTarget('flarm:000001', {
          position,
          altitudeMslMeters: 500,
          trackDegrees: 90,
        }),
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

/** Adds one sample about 33 m north of the previous sample. */
function applyTrailSample(trail: TrailStore, seconds: number, maxAltitudeMslMeters = 1_000) {
  trail.apply({
    topic: 'trail',
    value: {
      recordingStart: 0,
      sample: {
        unixMilliseconds: seconds * 1_000,
        position: { latitudeDegrees: 50.82 + seconds * 0.0003, longitudeDegrees: 6.18 },
        altitudeMslMeters: 1_000,
        altitudeAglMeters: 800,
        varioMetersPerSecond: 1,
        verticalSpeedMetersPerSecond: 0.5,
        nettoMetersPerSecond: 1,
        relativeVarioMetersPerSecond: 1,
        estimatedNettoMetersPerSecond: 1,
        estimatedRelativeVarioMetersPerSecond: 1,
      },
      maxAltitudeMslMeters,
      maxAltitudeAglMeters: 800,
    },
  });
}

it('renders the trail segments before and after the source exists', async () => {
  let trail = new TrailStore(new FakeClient());
  applyTrailSample(trail, 0);
  applyTrailSample(trail, 1);
  let mapState = new MapState();
  await render(MapComponent, { ...mapProps(mapState), trail });
  await vi.waitFor(() => expect(mapState.map?.getSource('trail')).toBeDefined());
  let source = mapState.map!.getSource<GeoJSONSource>('trail')!;

  applyTrailSample(trail, 2);

  await vi.waitFor(async () => {
    let data = await source.getData();
    expect(data.type === 'FeatureCollection' && data.features.map(({ id }) => id)).toEqual([
      1_000, 2_000,
    ]);
  });
});

/** Renders the trail with the colour mode of `settings`, as the root layout does. */
async function renderTrail(trail: TrailStore, settings: SettingsStore) {
  let mapState = new MapState();
  await render(MapComponent, {
    ...mapProps(mapState),
    trail,
    get trailColorMode() {
      return settings.current.trailColorMode;
    },
  });
  await vi.waitFor(() => expect(mapState.map?.getLayer('trail')).toBeDefined());
  let map = mapState.map!;
  let source = map.getSource<GeoJSONSource>('trail')!;
  await vi.waitFor(async () => {
    let data = await source.getData();
    expect(data.type === 'FeatureCollection' && data.features).toHaveLength(2);
  });
  return { map, source };
}

function applyTrailColorMode(settings: SettingsStore, trailColorMode: TrailColorMode) {
  settings.apply({ topic: 'settings', value: settingsFixture({ trailColorMode }) });
}

it('recolours the whole trail without a data change when the colour mode changes', async () => {
  let trail = new TrailStore(new FakeClient());
  for (let seconds of [0, 1, 2]) applyTrailSample(trail, seconds);
  let settings = new SettingsStore();
  let { map, source } = await renderTrail(trail, settings);
  expect(map.getPaintProperty('trail', 'line-color')).toEqual(trailColor('relativeVario', trail));
  let setData = vi.spyOn(source, 'setData');
  let updateData = vi.spyOn(source, 'updateData');

  applyTrailColorMode(settings, 'altitude');

  await vi.waitFor(() => {
    expect(map.getPaintProperty('trail', 'line-color')).toEqual(trailColor('altitude', trail));
  });
  expect(setData).not.toHaveBeenCalled();
  expect(updateData).not.toHaveBeenCalled();
});

it('recolours the older trail segments after a new maximum altitude', async () => {
  let trail = new TrailStore(new FakeClient());
  for (let seconds of [0, 1, 2]) applyTrailSample(trail, seconds);
  let settings = new SettingsStore();
  applyTrailColorMode(settings, 'altitude');
  let { map } = await renderTrail(trail, settings);

  applyTrailSample(trail, 3, 1_500);

  let maxima = { maxAltitudeMslMeters: 1_500, maxAltitudeAglMeters: 800 };
  await vi.waitFor(() => {
    expect(map.getPaintProperty('trail', 'line-color')).toEqual(trailColor('altitude', maxima));
  });
});

it('moves the whole trail by the drift translation', async () => {
  let trail = new TrailStore(new FakeClient());
  let wind = { directionDegrees: 270, speedMetersPerSecond: 10, stale: false };
  let derived = { ...EMPTY_DERIVED_INSTRUMENTS, wind };
  trail.apply({ topic: 'settings', value: settingsFixture({ trailDriftCompensation: true }) });
  trail.apply({ topic: 'instruments', value: instrumentsFixture({ derived }) });
  for (let seconds of [0, 1, 2]) applyTrailSample(trail, seconds);
  let { map } = await renderTrail(trail, new SettingsStore());
  let before = trail.translation!;
  for (let layer of ['trail-halo', 'trail']) {
    expect(map.getPaintProperty(layer, 'line-translate')).toEqual(trailTranslate(before));
  }

  applyTrailSample(trail, 3);

  expect(trail.translation!.eastMeters - before.eastMeters).toBeCloseTo(10);
  await vi.waitFor(() => {
    for (let layer of ['trail-halo', 'trail']) {
      expect(map.getPaintProperty(layer, 'line-translate')).toEqual(
        trailTranslate(trail.translation),
      );
    }
  });
});

it('renders trail segments that are shorter than a pixel', async () => {
  let trail = new TrailStore(new FakeClient());
  for (let seconds = 0; seconds <= 10; seconds++) applyTrailSample(trail, seconds);
  let mapState = new MapState();
  await render(MapComponent, { ...mapProps(mapState), trail });
  await vi.waitFor(() => expect(mapState.map?.getLayer('trail')).toBeDefined());
  let map = mapState.map!;

  map.jumpTo({ center: [6.18, 50.8215], zoom: 8 });
  await map.once('idle');

  let ids = map.queryRenderedFeatures({ layers: ['trail'] }).map(({ id }) => id);
  expect(new Set(ids).size).toBe(10);
});

it('publishes the map and camera values through the shared map state', async () => {
  let mapState = new MapState();
  await render(MapComponent, {
    ...mapProps(mapState),
    instruments,
    units,
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
    ...mapProps(mapState),
    instruments,
    traffic,
    units,
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

  map.fire('dragstart', { originalEvent: new MouseEvent('mousedown') });
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

it('updates the camera and ownship only when their values change', async () => {
  let { map, mapState, view } = await renderLoadedMap(positionInstruments);
  await vi.waitFor(() => expect(map.getSource('ownship')).toBeDefined());
  let source = map.getSource('ownship') as GeoJSONSource;
  await useFakeFrames();
  let camera = vi.spyOn(map, 'jumpTo');
  let ownship = vi.spyOn(source, 'setData');

  let updated = structuredClone(positionInstruments);
  updated.gps.altitudeMeters += 10;
  await view.rerender({ instruments: updated });
  await vi.advanceTimersByTimeAsync(100);
  expect(camera).not.toHaveBeenCalled();
  expect(ownship).not.toHaveBeenCalled();

  updated = structuredClone(updated);
  updated.gps.trackDegrees = 180;
  await view.rerender({ instruments: updated });
  await vi.advanceTimersByTimeAsync(100);
  expect(camera).not.toHaveBeenCalled();
  expect(ownship).toHaveBeenCalledTimes(1);
  expect(ownship.mock.lastCall?.[0]).toMatchObject({ properties: { track: 180 } });

  for (let coordinate of ['latitudeDegrees', 'longitudeDegrees'] as const) {
    camera.mockClear();
    ownship.mockClear();
    updated = structuredClone(updated);
    updated.gps.position[coordinate] += 0.01;
    await view.rerender({ instruments: updated });
    await vi.advanceTimersByTimeAsync(100);
    let center = [updated.gps.position.longitudeDegrees, updated.gps.position.latitudeDegrees];
    expect(camera).toHaveBeenCalledExactlyOnceWith({ center });
    expect(ownship).toHaveBeenCalledTimes(1);
    expect(ownship.mock.lastCall?.[0]).toMatchObject({ geometry: { coordinates: center } });
  }

  camera.mockClear();
  ownship.mockClear();
  map.fire('dragstart', { originalEvent: new MouseEvent('mousedown') });
  map.jumpTo({ center: [7, 51] });
  camera.mockClear();
  await view.rerender({ instruments: structuredClone(updated) });
  await vi.advanceTimersByTimeAsync(100);
  expect(mapState.followMode).toBe(false);
  expect(camera).not.toHaveBeenCalled();
  expect(ownship).not.toHaveBeenCalled();
});

it('follows the dead-reckoned ownship position for 1.5 seconds', async () => {
  let { map, view } = await renderLoadedMap();
  await useFakeFrames();
  await view.rerender({ instruments: movingInstruments });
  await vi.advanceTimersByTimeAsync(20);
  let source = map.getSource('ownship') as GeoJSONSource;
  let ownship = vi.spyOn(source, 'setData');

  // Fake frames run every 16 milliseconds after the receive time.
  await vi.advanceTimersByTimeAsync(476);
  expectCenter(map, reckonedCenter(0.496));

  await vi.advanceTimersByTimeAsync(1500);
  expectCenter(map, reckonedCenter(1.5));
  let [longitude, latitude] = reckonedCenter(1.5);
  let { geometry, properties } = ownship.mock.lastCall?.[0] as GeoJSON.Feature<GeoJSON.Point>;
  expect(geometry.coordinates[0]).toBeCloseTo(longitude, 9);
  expect(geometry.coordinates[1]).toBeCloseTo(latitude, 9);
  expect(properties?.track).toBeCloseTo(120, 9);

  let camera = vi.spyOn(map, 'jumpTo');
  ownship.mockClear();
  await vi.advanceTimersByTimeAsync(1000);
  expect(camera).not.toHaveBeenCalled();
  expect(ownship).not.toHaveBeenCalled();
});

it('eases to the ownship over 300 milliseconds when it returns to position', async () => {
  let { map, mapState } = await renderLoadedMap(positionInstruments);
  let position = positionInstruments.gps.position;
  let target: [number, number] = [position.longitudeDegrees, position.latitudeDegrees];
  await vi.waitFor(() => expectCenter(map, target));
  map.fire('dragstart', { originalEvent: new MouseEvent('mousedown') });
  map.jumpTo({ center: [7, 51] });
  let returnButton = page.getByRole('button', { name: 'Return to position' });
  await expect.element(returnButton).toBeVisible();
  await useFakeFrames();

  returnButton.element().dispatchEvent(new MouseEvent('click', { bubbles: true }));
  expect(mapState.followMode).toBe(true);

  // The ease moves a cubic ease-out fraction of the way at the frame after 144 milliseconds.
  await vi.advanceTimersByTimeAsync(144);
  let eased = 1 - (1 - 144 / 300) ** 3;
  expectCenter(map, [7 + (target[0] - 7) * eased, 51 + (target[1] - 51) * eased]);

  await vi.advanceTimersByTimeAsync(200);
  expectCenter(map, target);
});

it('keeps following during zoom and rotation and stops for a drag', async () => {
  let { map, mapState } = await renderLoadedMap(positionInstruments);
  let touch = (identifier: number) => new Touch({ identifier, target: map.getCanvas() });

  for (let originalEvent of [new WheelEvent('wheel'), new MouseEvent('mousedown', { button: 2 })]) {
    map.fire('movestart', { originalEvent });
    expect(mapState.followMode).toBe(true);
  }
  // A pinch zoom also pans with the center of its two touches.
  let pinch = new TouchEvent('touchstart', { touches: [touch(1), touch(2)] });
  map.fire('dragstart', { originalEvent: pinch });
  expect(mapState.followMode).toBe(true);

  for (let originalEvent of [
    new MouseEvent('mousedown'),
    new TouchEvent('touchstart', { touches: [touch(1)] }),
  ]) {
    mapState.followMode = true;
    map.fire('dragstart', { originalEvent });
    expect(mapState.followMode).toBe(false);
  }
});

it('stops following for a drag in a browser without touch events', async () => {
  let { map, mapState } = await renderLoadedMap(positionInstruments);
  vi.stubGlobal('TouchEvent', undefined);
  try {
    map.fire('dragstart', { originalEvent: new MouseEvent('mousedown') });
  } finally {
    vi.unstubAllGlobals();
  }

  expect(mapState.followMode).toBe(false);
});

it('does not cancel a zoom animation while it follows the ownship', async () => {
  let { map, view } = await renderLoadedMap();
  await useFakeFrames();
  await view.rerender({ instruments: movingInstruments });
  await vi.advanceTimersByTimeAsync(20);
  let camera = vi.spyOn(map, 'jumpTo');

  map.zoomTo(13, { duration: 300 });
  await vi.advanceTimersByTimeAsync(100);
  expect(map.isZooming()).toBe(true);
  expect(camera).not.toHaveBeenCalled();

  await vi.advanceTimersByTimeAsync(400);
  expect(map.getZoom()).toBe(13);
  expect(camera).toHaveBeenCalled();
});

it('zooms around the center while it follows the ownship', async () => {
  let { map, mapState } = await renderLoadedMap(positionInstruments);
  let position = positionInstruments.gps.position;
  let target: [number, number] = [position.longitudeDegrees, position.latitudeDegrees];
  await vi.waitFor(() => expectCenter(map, target));
  let touchZoom = vi.spyOn(map.touchZoomRotate, 'enable');
  await useFakeFrames();

  let canvas = map.getCanvas();
  let { left, top } = canvas.getBoundingClientRect();
  canvas.dispatchEvent(
    new WheelEvent('wheel', { deltaY: -200, clientX: left + 10, clientY: top + 10, bubbles: true }),
  );
  await vi.advanceTimersByTimeAsync(48);
  expect(map.isZooming()).toBe(true);
  expect(map.getZoom()).not.toBe(11);
  expectCenter(map, target);

  mapState.followMode = false;
  await vi.advanceTimersByTimeAsync(16);
  expect(touchZoom).toHaveBeenLastCalledWith(undefined);
  mapState.followMode = true;
  await vi.advanceTimersByTimeAsync(16);
  expect(touchZoom).toHaveBeenLastCalledWith({ around: 'center' });
});

it('does not reset a gesture before it starts', async () => {
  let { map, view } = await renderLoadedMap();
  await useFakeFrames();
  await view.rerender({ instruments: movingInstruments });
  await vi.advanceTimersByTimeAsync(16);
  let camera = vi.spyOn(map, 'jumpTo');

  map
    .getCanvasContainer()
    .dispatchEvent(new PointerEvent('pointerdown', { pointerId: 1, bubbles: true }));
  await vi.advanceTimersByTimeAsync(200);
  expect(camera).not.toHaveBeenCalled();

  window.dispatchEvent(new PointerEvent('pointerup', { pointerId: 1 }));
  await vi.advanceTimersByTimeAsync(16);
  expect(camera).toHaveBeenCalled();
});

it('keeps the dead-reckoned ownship when the position becomes stale', async () => {
  let { map, view } = await renderLoadedMap();
  await useFakeFrames();
  await view.rerender({ instruments: movingInstruments });
  await vi.advanceTimersByTimeAsync(2000);
  expectCenter(map, reckonedCenter(1.5));
  let ownship = vi.spyOn(map.getSource('ownship') as GeoJSONSource, 'setData');

  let stale = structuredClone(movingInstruments);
  stale.gps.stale = true;
  await view.rerender({ instruments: stale });
  await vi.advanceTimersByTimeAsync(100);
  expectCenter(map, reckonedCenter(1.5));
  expect(ownship).not.toHaveBeenCalled();
});

it('centers the ownship again after a camera movement ends', async () => {
  let { map } = await renderLoadedMap(positionInstruments);
  let position = positionInstruments.gps.position;
  let target: [number, number] = [position.longitudeDegrees, position.latitudeDegrees];
  await vi.waitFor(() => expectCenter(map, target));
  await useFakeFrames();

  map.zoomTo(12, { duration: 200, around: [7, 51] });
  await vi.advanceTimersByTimeAsync(400);
  expect(map.getZoom()).toBe(12);
  expectCenter(map, target);
});

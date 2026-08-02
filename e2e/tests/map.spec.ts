import type { Page } from '@playwright/test';
import type { GeoJSONSource, Map as MapLibreMap } from 'maplibre-gl';

import { expect, test } from '@playwright/test';

import { AIRSPACE_BROWSER_FIXTURE } from '../../frontend/src/lib/map/airspace.fixture';

type Instruments = {
  position: { latitudeDegrees: number; longitudeDegrees: number };
  trackDegrees: number;
  groundSpeedMetersPerSecond: number;
  altitudeMslMeters: number;
};

type MapState = {
  center: number[];
  renderedCoordinates: number[] | null;
  sourceCoordinates: number[];
};

type AirspaceMapState = {
  featureCount: number;
  layerOrder: string[];
  renderedLayerIds: string[];
};

type TestWindow = Window & {
  __updraftTest?: { map: MapLibreMap };
  __updraftFake?: { emit: (topic: unknown) => void };
  __updraftTestAirspaceData?: typeof AIRSPACE_BROWSER_FIXTURE;
};

const POSITION_A: Instruments = {
  position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
  trackDegrees: 45,
  groundSpeedMetersPerSecond: 30,
  altitudeMslMeters: 400,
};

const POSITION_B: Instruments = {
  position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
  trackDegrees: 90,
  groundSpeedMetersPerSecond: 31,
  altitudeMslMeters: 410,
};

test('renders the ownship position and follows live updates', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await expectMapPosition(page, POSITION_A);

  await emitInstruments(page, POSITION_B);
  await expectMapPosition(page, POSITION_B);
});

test('renders active airspace below traffic and ownship', async ({ page }) => {
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, AIRSPACE_BROWSER_FIXTURE);
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake?.emit({
      topic: 'airspace',
      value: {
        type: 'active',
        sourceName: 'browser-fixture.txt',
        airspaceCount: 2,
        generation: 1,
      },
    });
  });

  await expect
    .poll(() => readAirspaceMapState(page), { message: 'map to render both airspace layers' })
    .toEqual({
      featureCount: 2,
      layerOrder: [
        'airspace-fill',
        'airspace-outline',
        'traffic-fixed',
        'traffic-directional',
        'ownship-symbol',
      ],
      renderedLayerIds: ['airspace-fill', 'airspace-outline'],
    });
});

async function emitInstruments(page: Page, instruments: Instruments) {
  await page.evaluate((value) => {
    (window as TestWindow).__updraftFake?.emit({ topic: 'instruments', value });
  }, instruments);
}

async function expectMapPosition(page: Page, instruments: Instruments) {
  let { latitudeDegrees, longitudeDegrees } = instruments.position;

  await expect
    .poll(() => readMapState(page), {
      message: `map to render position ${latitudeDegrees}, ${longitudeDegrees}`,
    })
    .toEqual({
      center: [expect.closeTo(longitudeDegrees, 6), expect.closeTo(latitudeDegrees, 6)],
      renderedCoordinates: [
        expect.closeTo(longitudeDegrees, 4),
        expect.closeTo(latitudeDegrees, 4),
      ],
      sourceCoordinates: [expect.closeTo(longitudeDegrees, 6), expect.closeTo(latitudeDegrees, 6)],
    });
}

async function readMapState(page: Page): Promise<MapState | null> {
  return page.evaluate(async () => {
    let map = (window as TestWindow).__updraftTest?.map;
    let source = map?.getSource<GeoJSONSource>('ownship');
    if (!map || !source) return null;

    let data = await source.getData();
    if (data.type !== 'Feature' || data.geometry?.type !== 'Point') return null;

    let center = map.getCenter();
    let renderedOwnship = map.queryRenderedFeatures({ layers: ['ownship-symbol'] })[0];

    return {
      center: [center.lng, center.lat],
      renderedCoordinates:
        renderedOwnship?.geometry.type === 'Point' ? renderedOwnship.geometry.coordinates : null,
      sourceCoordinates: data.geometry.coordinates,
    };
  });
}

async function readAirspaceMapState(page: Page): Promise<AirspaceMapState | null> {
  return page.evaluate(async () => {
    let map = (window as TestWindow).__updraftTest?.map;
    let source = map?.getSource<GeoJSONSource>('airspace');
    if (!map || !source || !map.isSourceLoaded('airspace')) return null;

    let data = await source.getData();
    if (data.type !== 'FeatureCollection') return null;

    let airspaceLayerIds = ['airspace-fill', 'airspace-outline'];
    if (airspaceLayerIds.some((id) => !map.getLayer(id))) return null;

    let relevantLayerIds = new Set([
      ...airspaceLayerIds,
      'traffic-fixed',
      'traffic-directional',
      'ownship-symbol',
    ]);
    let layerOrder = (map.getStyle().layers ?? [])
      .map((layer) => layer.id)
      .filter((id) => relevantLayerIds.has(id));
    let renderedLayerIds = [
      ...new Set(
        map.queryRenderedFeatures({ layers: airspaceLayerIds }).map((feature) => feature.layer.id),
      ),
    ].sort();

    return {
      featureCount: data.features.length,
      layerOrder,
      renderedLayerIds,
    };
  });
}

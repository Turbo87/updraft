import type { Page } from '@playwright/test';
import type { GeoJSONSource } from 'maplibre-gl';
import type { AppContext } from '$lib/app-context';
import type { GpsInstruments } from '$lib/protocol/generated/GpsInstruments';

import { expect, test } from '@playwright/test';

import { AIRSPACE_BROWSER_FIXTURE } from '../../frontend/src/lib/map/airspace.fixture';

type MapState = {
  center: number[];
  renderedCoordinates: number[] | null;
  sourceCoordinates: number[];
};

type MapCenter = {
  latitudeDegrees: number;
  longitudeDegrees: number;
};

type AirspaceMapState = {
  featureCount: number;
  layerOrder: string[];
  renderedLayerIds: string[];
};

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: { emit: (topic: unknown) => void };
  __updraftTestAirspaceData?: typeof AIRSPACE_BROWSER_FIXTURE;
};

const POSITION_A: GpsInstruments = {
  position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
  trackDegrees: 45,
  groundSpeedMetersPerSecond: 30,
  altitudeMeters: 400,
  fixTime: null,
  stale: false,
};

const POSITION_B: GpsInstruments = {
  position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
  trackDegrees: 90,
  groundSpeedMetersPerSecond: 31,
  altitudeMeters: 410,
  fixTime: null,
  stale: false,
};

const POSITION_C: GpsInstruments = {
  position: { latitudeDegrees: 50.825, longitudeDegrees: 6.188 },
  trackDegrees: 135,
  groundSpeedMetersPerSecond: 32,
  altitudeMeters: 420,
  fixTime: null,
  stale: false,
};

test('follows live positions until the user pans and returns', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await expectMapPosition(page, POSITION_A);

  let followedCenter = await readMapCenter(page);
  await panMap(page);
  let returnButton = page.getByRole('button', { name: 'Return to position' });
  await expect(returnButton).toBeVisible();
  let manualCenter = await expectFullPan(page, followedCenter);

  await panMap(page);
  manualCenter = await expectFullPan(page, manualCenter);

  await emitInstruments(page, POSITION_B);
  await expectMapPosition(page, POSITION_B, manualCenter);

  await returnButton.click();
  await expect(returnButton).not.toBeVisible();
  await expectMapPosition(page, POSITION_B);

  await emitInstruments(page, POSITION_C);
  await expectMapPosition(page, POSITION_C);
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

test('opens a tapped map position and updates its ownship relation', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await expectMapPosition(page, POSITION_A);

  let bounds = await page.locator('.maplibregl-canvas').boundingBox();
  if (!bounds) throw new Error('Map canvas is not visible');

  await page.mouse.click(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await expect(page).toHaveURL('/nearby/50.823000/6.186000');
  await expect(page.getByRole('heading', { name: 'Nearby' })).toBeVisible();
  await expect(page.getByText('50.82300, 6.18600')).toBeVisible();
  await expect(page.getByText('0.0 km', { exact: true })).toBeVisible();
  await expect(page.getByText('0°', { exact: true })).toBeVisible();

  await emitInstruments(page, POSITION_B);
  await expect(page.getByText('0.1 km', { exact: true })).toBeVisible();
  await expect(page.getByText('212°', { exact: true })).toBeVisible();

  await page.getByRole('link', { name: 'Back to map' }).click();
  await expect(page).toHaveURL('/');

  await page.goto('/nearby/91/6.186?testMode=1');
  await expect(page.getByText('The selected map position is invalid.')).toBeVisible();
  await page.getByRole('link', { name: 'Back to map' }).click();
  await expect(page).toHaveURL('/');
});

async function emitInstruments(page: Page, gps: GpsInstruments) {
  await page.evaluate(
    (value) => {
      (window as TestWindow).__updraftFake?.emit({ topic: 'instruments', value });
    },
    { gps, pressureAltitude: null },
  );
}

async function expectMapPosition(
  page: Page,
  instruments: GpsInstruments,
  expectedCenter = instruments.position,
) {
  let { latitudeDegrees, longitudeDegrees } = instruments.position;

  await expect
    .poll(() => readMapState(page), {
      message: `map to render position ${latitudeDegrees}, ${longitudeDegrees}`,
    })
    .toEqual({
      center: [
        expect.closeTo(expectedCenter.longitudeDegrees, 6),
        expect.closeTo(expectedCenter.latitudeDegrees, 6),
      ],
      renderedCoordinates: [
        expect.closeTo(longitudeDegrees, 4),
        expect.closeTo(latitudeDegrees, 4),
      ],
      sourceCoordinates: [expect.closeTo(longitudeDegrees, 6), expect.closeTo(latitudeDegrees, 6)],
    });
}

async function panMap(page: Page) {
  let bounds = await page.locator('.maplibregl-canvas').boundingBox();
  if (!bounds) throw new Error('Map canvas is not visible');

  let centerX = bounds.x + bounds.width / 2;
  let centerY = bounds.y + bounds.height / 2;
  await page.mouse.move(centerX, centerY);
  await page.mouse.down();
  await page.mouse.move(centerX + 120, centerY, { steps: 5 });
  await page.mouse.up();
}

async function readMapCenter(page: Page): Promise<MapCenter> {
  return page.evaluate(() => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    if (!map) throw new Error('Map is not available');

    let center = map.getCenter();
    return { latitudeDegrees: center.lat, longitudeDegrees: center.lng };
  });
}

async function expectFullPan(page: Page, previousCenter: MapCenter): Promise<MapCenter> {
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp?.mapState.map?.isMoving()))
    .toBe(false);

  let { center, displacementPixels } = await page.evaluate((previous) => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    if (!map) throw new Error('Map is not available');

    let current = map.getCenter();
    let previousPoint = map.project([previous.longitudeDegrees, previous.latitudeDegrees]);
    let currentPoint = map.project(current);
    return {
      center: { latitudeDegrees: current.lat, longitudeDegrees: current.lng },
      displacementPixels: Math.abs(previousPoint.x - currentPoint.x),
    };
  }, previousCenter);
  expect(displacementPixels).toBeGreaterThan(100);
  return center;
}

async function readMapState(page: Page): Promise<MapState | null> {
  return page.evaluate(async () => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
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
    let map = (window as TestWindow).__updraftApp?.mapState.map;
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

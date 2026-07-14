import { expect, test, type Page } from '@playwright/test';
import type { GeoJSONSource, Map as MapLibreMap } from 'maplibre-gl';

type Position = {
  observedAtMs: number;
  latitudeDegrees: number;
  longitudeDegrees: number;
  altitudeMeters: number;
  trackDegrees: number;
  groundSpeedMetersPerSecond: number;
};

type MapState = {
  center: number[];
  renderedCoordinates: number[] | null;
  sourceCoordinates: number[];
};

type TestWindow = Window & {
  __updraftTest?: { map: MapLibreMap };
};

const POSITION_A: Position = {
  observedAtMs: 1_000,
  latitudeDegrees: 50.823,
  longitudeDegrees: 6.186,
  altitudeMeters: 400,
  trackDegrees: 45,
  groundSpeedMetersPerSecond: 30,
};

const POSITION_B: Position = {
  observedAtMs: 2_000,
  latitudeDegrees: 50.824,
  longitudeDegrees: 6.187,
  altitudeMeters: 410,
  trackDegrees: 90,
  groundSpeedMetersPerSecond: 31,
};

test('renders the position snapshot and live updates', async ({ page, request }) => {
  let response = await request.post('/api/simulation/position', { data: POSITION_A });
  expect(response.status()).toBe(204);

  await page.goto('/?testMode=1');
  await expectMapPosition(page, POSITION_A);

  response = await request.post('/api/simulation/position', { data: POSITION_B });
  expect(response.status()).toBe(204);

  await expectMapPosition(page, POSITION_B);
});

async function expectMapPosition(page: Page, position: Position) {
  await expect
    .poll(() => readMapState(page), {
      message: `map to render position ${position.latitudeDegrees}, ${position.longitudeDegrees}`,
    })
    .toEqual({
      center: [
        expect.closeTo(position.longitudeDegrees, 6),
        expect.closeTo(position.latitudeDegrees, 6),
      ],
      renderedCoordinates: [
        expect.closeTo(position.longitudeDegrees, 4),
        expect.closeTo(position.latitudeDegrees, 4),
      ],
      sourceCoordinates: [
        expect.closeTo(position.longitudeDegrees, 6),
        expect.closeTo(position.latitudeDegrees, 6),
      ],
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

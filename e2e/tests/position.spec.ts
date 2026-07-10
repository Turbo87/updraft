import { expect, test } from 'playwright/test';
import type { Page } from 'playwright/test';

// Minimal structural view of the `window.updraftTest` map handle exposed
// by the frontend in test mode; keeps this package free of maplibre types.
interface SourceView {
  serialize(): { data: unknown };
}

interface MapView {
  loaded(): boolean;
  getSource(id: string): SourceView | undefined;
  once(event: 'idle', listener: () => void): void;
  queryRenderedFeatures(options: { layers: string[] }): unknown[];
  triggerRepaint(): void;
}

interface TestWindow extends Window {
  updraftTest?: { map: MapView };
}

function readOwnshipCoordinates(page: Page) {
  return page.evaluate(() => {
    let source = (window as TestWindow).updraftTest?.map.getSource('ownship');
    let data = source?.serialize().data;
    if (typeof data !== 'object' || data === null || !('geometry' in data)) {
      return null;
    }
    let geometry = data.geometry;
    if (typeof geometry !== 'object' || geometry === null || !('coordinates' in geometry)) {
      return null;
    }
    return geometry.coordinates;
  });
}

function awaitMapIdle(page: Page) {
  return page.evaluate(
    () =>
      new Promise<void>((resolve, reject) => {
        let map = (window as TestWindow).updraftTest?.map;
        if (!map) {
          reject(new Error('test map is not available'));
          return;
        }
        map.once('idle', resolve);
        map.triggerRepaint();
      }),
  );
}

test('renders positions submitted through the simulation seam', async ({ page, request }) => {
  await page.goto('/?testMode=1');

  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).updraftTest?.map.loaded() ?? false))
    .toBe(true);

  // Simulated timestamps are injected, so e2e time is controlled rather
  // than wall time (see docs/design/testing.md).
  let response = await request.post('/api/simulation/position', {
    data: { latitude: 50.823, longitude: 6.186, track: 45, observed_at_ms: 1_000 },
  });
  expect(response.status()).toBe(204);

  await expect.poll(() => readOwnshipCoordinates(page)).toEqual([6.186, 50.823]);

  response = await request.post('/api/simulation/position', {
    data: { latitude: 50.824, longitude: 6.187, track: 90, observed_at_ms: 2_000 },
  });
  expect(response.status()).toBe(204);

  await expect.poll(() => readOwnshipCoordinates(page)).toEqual([6.187, 50.824]);

  // The flown distance comes back from the compute worker (~132 m
  // between the two fixes) and reaches the UI through the state stream.
  await expect(page.locator('.track-distance')).toHaveText('0.1 km');

  await awaitMapIdle(page);

  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as TestWindow).updraftTest?.map.queryRenderedFeatures({
            layers: ['ownship-symbol'],
          }).length ?? 0,
      ),
    )
    .toBeGreaterThan(0);
});

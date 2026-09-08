import type { Route } from '@playwright/test';
import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: FakeClient;
  __TAURI_INTERNALS__?: { convertFileSrc: (path: string) => string };
};

// A vector tile with one point at its centre in the "test" layer.
function pointTile(id: number): Buffer {
  let tile = Buffer.from('1a180a0474657374120b08001801220509802080202880207802', 'hex');
  tile[11] = id;
  return tile;
}

test('activation and removal refresh tiles without moving the map', async ({ page }) => {
  let pending: Route[] = [];
  let cancelled = 0;
  page.on('requestfailed', (request) => {
    if (request.url().includes('/basemap/1/')) cancelled++;
  });
  await page.route('**/__test/basemap/**', async (route) => {
    if (route.request().url().includes('/basemap/1/')) {
      pending.push(route);
      return;
    }
    await route.fulfill({
      contentType: 'application/vnd.mapbox-vector-tile',
      body: route.request().url().includes('/basemap/3/')
        ? Buffer.alloc(0)
        : pointTile(route.request().url().includes('/basemap/0/') ? 7 : 9),
    });
  });
  await page.goto('/?testMode=1');
  await page.waitForFunction(() =>
    (window as TestWindow).__updraftApp?.mapState.map?.getLayer('traffic-fixed'),
  );
  let initial = await page.evaluate(() => {
    let app = (window as TestWindow).__updraftApp!;
    let map = app.mapState.map!;
    app.mapState.followMode = false;
    map.jumpTo({ center: [0, 0], zoom: 6, bearing: 12, pitch: 20 });
    (window as TestWindow).__TAURI_INTERNALS__ = {
      convertFileSrc: (path) => `${location.origin}/__test/${path}`,
    };
    (window as TestWindow).__updraftFake!.emitBasemaps({
      generation: 0,
      sources: [{ sourceName: 'local.mbtiles', type: 'active' }],
    });
    map.addSource('openmaptiles', {
      type: 'vector',
      tiles: [`${location.origin}/__test/basemap/0/{z}/{x}/{y}.pbf`],
      minzoom: 6,
      maxzoom: 6,
    });
    for (let [id, before] of [['basemap-before', 'traffic-fixed'], ['basemap-after']] as const) {
      map.addLayer({ id, type: 'circle', source: 'openmaptiles', 'source-layer': 'test' }, before);
    }
    return {
      center: map.getCenter().toArray(),
      zoom: map.getZoom(),
      bearing: map.getBearing(),
      pitch: map.getPitch(),
      layers: map.getStyle().layers.map((layer) => layer.id),
    };
  });
  function features() {
    return page.evaluate(() => {
      let map = (window as TestWindow).__updraftApp!.mapState.map!;
      return [
        ...new Set(
          map
            .querySourceFeatures('openmaptiles', { sourceLayer: 'test' })
            .map((feature) => feature.id),
        ),
      ];
    });
  }
  await expect.poll(features).toEqual([7]);
  await page.getByRole('link', { name: 'Settings', exact: true }).click();
  await page.getByRole('link', { name: 'Data', exact: true }).click();
  await page.getByRole('button', { name: /^local / }).click();
  let toggle = page.getByRole('switch', { name: 'Enabled' });
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await expect.poll(() => pending.length).toBeGreaterThan(0);
  await expect.poll(features).toEqual([]);
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect.poll(features).toEqual([9]);
  await expect.poll(() => cancelled).toBe(pending.length);
  for (let route of pending) await route.fulfill({ body: pointTile(7) });
  await expect.poll(features).toEqual([9]);
  await page.getByRole('button', { name: 'Remove from device' }).click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect.poll(features).toEqual([9]);
  await page.getByRole('button', { name: /^local / }).click();
  await page.getByRole('button', { name: 'Remove from device' }).click();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  await expect(page.getByText('No data on this device')).toBeVisible();
  await expect.poll(features).toEqual([]);
  let final = await page.evaluate(() => {
    let map = (window as TestWindow).__updraftApp!.mapState.map!;
    return {
      center: map.getCenter().toArray(),
      zoom: map.getZoom(),
      bearing: map.getBearing(),
      pitch: map.getPitch(),
      layers: map.getStyle().layers.map((layer) => layer.id),
    };
  });
  expect(final).toEqual(initial);
});

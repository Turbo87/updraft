import type { Route } from '@playwright/test';
import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: FakeClient;
  __TAURI_INTERNALS__?: { convertFileSrc: (path: string) => string };
};

for (let pendingResource of ['metadata', 'tiles'] as const) {
  test(`activation replaces terrain with pending ${pendingResource} without moving the map`, async ({
    page,
  }) => {
    let errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    let pending: Route[] = [];
    let cancelled = 0;
    let tilesBySize: Buffer[];
    let tiles = new Set<number>();
    page.on('requestfailed', (request) => {
      if (request.url().includes('/terrain/1/')) cancelled++;
    });
    await page.route('**/__test/terrain/**', async (route) => {
      let url = route.request().url();
      let generation = Number(url.split('/terrain/')[1].split('/')[0]);
      if (generation === 1 && (pendingResource === 'metadata' || !url.endsWith('metadata.json'))) {
        pending.push(route);
        return;
      }
      if (url.endsWith('metadata.json')) {
        await route.fulfill({
          json: {
            tilejson: '3.0.0',
            encoding: 'terrarium',
            tileSize: generation === 0 ? 256 : 512,
            minzoom: 6,
            maxzoom: 6,
            attribution: generation === 0 ? 'Initial terrain credit' : 'Replacement terrain credit',
            tiles: [`${new URL(url).origin}/__test/terrain/${generation}/{z}/{x}/{y}.webp`],
          },
        });
      } else {
        tiles.add(generation);
        await route.fulfill({
          contentType: 'image/webp',
          body: tilesBySize[generation === 0 ? 0 : 1],
        });
      }
    });
    await page.goto('/?testMode=1');
    await page.waitForFunction(() =>
      (window as TestWindow).__updraftApp?.mapState.map?.getLayer('traffic-fixed'),
    );
    let encoded = await page.evaluate(() =>
      [256, 512].map((size) => {
        let canvas = document.createElement('canvas');
        canvas.width = canvas.height = size;
        let context = canvas.getContext('2d')!;
        context.fillStyle = 'rgb(128, 0, 0)';
        context.fillRect(0, 0, size, size);
        return canvas.toDataURL('image/webp', 1).split(',')[1];
      }),
    );
    tilesBySize = encoded.map((data) => Buffer.from(data, 'base64'));
    let initial = await page.evaluate(() => {
      let app = (window as TestWindow).__updraftApp!;
      let map = app.mapState.map!;
      app.mapState.followMode = false;
      map.jumpTo({ center: [0, 0], zoom: 6, bearing: 12, pitch: 20 });
      (window as TestWindow).__TAURI_INTERNALS__ = {
        convertFileSrc: (path) => `${location.origin}/__test/${path}`,
      };
      (window as TestWindow).__updraftFake!.emitTerrain({
        generation: 0,
        sources: [{ sourceName: 'local.terrain', type: 'active' }],
      });
      map.addSource('terrain', {
        type: 'raster-dem',
        url: `${location.origin}/__test/terrain/0/metadata.json`,
        tiles: [`${location.origin}/__test/terrain/0/{z}/{x}/{y}.webp`],
      });
      map.addLayer(
        { id: 'terrain-hillshade', type: 'hillshade', source: 'terrain' },
        'traffic-fixed',
      );
      map.addLayer({ id: 'terrain-color-relief', type: 'color-relief', source: 'terrain' });
      return {
        center: map.getCenter().toArray(),
        zoom: map.getZoom(),
        bearing: map.getBearing(),
        pitch: map.getPitch(),
        layers: map.getStyle().layers.map((layer) => layer.id),
      };
    });
    async function metadata() {
      return page.evaluate(() => {
        let source = (window as TestWindow).__updraftApp!.mapState.map!.getSource('terrain')!;
        return {
          attribution: source.attribution,
          tileSize: 'tileSize' in source ? source.tileSize : null,
        };
      });
    }
    await expect.poll(metadata).toEqual({ attribution: 'Initial terrain credit', tileSize: 256 });
    await expect.poll(() => [...tiles]).toEqual([0]);
    await page.getByRole('link', { name: 'Settings', exact: true }).click();
    await page.getByRole('link', { name: 'Data', exact: true }).click();
    await page.getByRole('button', { name: /^local.terrain/ }).click();
    let toggle = page.getByRole('switch', { name: 'Enabled' });
    await toggle.click();
    await expect(toggle).not.toBeChecked();
    await expect.poll(() => pending.length).toBeGreaterThan(0);
    await expect
      .poll(async () => (await metadata()).attribution)
      .not.toBe('Initial terrain credit');
    await toggle.click();
    await expect(toggle).toBeChecked();
    await expect
      .poll(metadata)
      .toEqual({ attribution: 'Replacement terrain credit', tileSize: 512 });
    await expect.poll(() => [...tiles]).toEqual([0, 2]);
    await expect.poll(() => cancelled).toBe(pending.length);
    for (let route of pending) await route.abort();
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
    await page.getByRole('button', { name: 'Close', exact: true }).click();
    await page.getByRole('link', { name: 'Back to Settings' }).click();
    await page.getByRole('link', { name: 'About' }).click();
    await expect(page.getByText('Replacement terrain credit', { exact: true })).toBeVisible();
    await expect(page.getByText('Initial terrain credit', { exact: true })).toHaveCount(0);
    expect(errors).toEqual([]);
  });
}

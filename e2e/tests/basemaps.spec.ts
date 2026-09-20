import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: FakeClient;
};

test('activation and removal publish basemap generations without moving the map', async ({
  page,
}) => {
  await page.goto('/?testMode=1');
  await page.waitForFunction(() =>
    (window as TestWindow).__updraftApp?.mapState.map?.getLayer('traffic-fixed'),
  );
  let initial = await page.evaluate(() => {
    let app = (window as TestWindow).__updraftApp!;
    app.mapState.followMode = false;
    app.mapState.map!.jumpTo({ center: [0, 0], zoom: 6, bearing: 12, pitch: 20 });
    (window as TestWindow).__updraftFake!.emitBasemaps({
      generation: 0,
      sources: [{ sourceName: 'local.mbtiles', type: 'active' }],
    });
    return {
      center: app.mapState.center,
      zoom: app.mapState.zoom,
      bearing: app.mapState.bearing,
      pitch: app.mapState.pitch,
    };
  });
  async function status() {
    return page.evaluate(() => (window as TestWindow).__updraftApp!.basemaps.current);
  }
  await page.getByRole('link', { name: 'Settings', exact: true }).click();
  await page.getByRole('link', { name: 'Data', exact: true }).click();
  await page.getByRole('button', { name: /^local / }).click();
  let toggle = page.getByRole('switch', { name: 'Enabled' });
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await expect
    .poll(status)
    .toEqual({ generation: 1, sources: [{ sourceName: 'local.mbtiles', type: 'disabled' }] });
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect
    .poll(status)
    .toEqual({ generation: 2, sources: [{ sourceName: 'local.mbtiles', type: 'active' }] });
  await page.getByRole('button', { name: 'Remove from device' }).click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(await status()).toEqual({
    generation: 2,
    sources: [{ sourceName: 'local.mbtiles', type: 'active' }],
  });
  await page.getByRole('button', { name: /^local / }).click();
  await page.getByRole('button', { name: 'Remove from device' }).click();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  await expect(page.getByText('No data on this device')).toBeVisible();
  await expect.poll(status).toEqual({ generation: 3, sources: [] });
  let final = await page.evaluate(() => {
    let state = (window as TestWindow).__updraftApp!.mapState;
    return { center: state.center, zoom: state.zoom, bearing: state.bearing, pitch: state.pitch };
  });
  expect(final).toEqual(initial);
});

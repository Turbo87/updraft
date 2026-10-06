import { expect } from '@playwright/test';

import { pinnedTarget, test } from './app';

const mapPosition = {
  type: 'mapPosition',
  latitudeDegrees: 50.823,
  longitudeDegrees: 6.186,
} as const;
const home = {
  type: 'waypoint',
  name: 'Home',
  latitudeDegrees: 50,
  longitudeDegrees: 6,
  elevationMeters: 100,
} as const;

for (let viewport of [
  { width: 390, height: 844 },
  { width: 844, height: 390 },
]) {
  test(`selects and stops a map target at ${viewport.width}x${viewport.height}`, async ({
    page,
    app,
  }) => {
    await page.setViewportSize(viewport);
    await app.open('/nearby/50.823/6.186');
    await page.waitForFunction(() => window.__updraftApp?.mapState.map?.isStyleLoaded());
    let camera = await page.evaluate(() => {
      let state = window.__updraftApp!.mapState;
      return {
        zoom: state.map!.getZoom(),
        bearing: state.map!.getBearing(),
        follow: state.followMode,
      };
    });
    await page.getByRole('button', { name: 'Navigate here' }).click();
    await expect(page).toHaveURL('/');
    expect(await app.navigationCommands()).toEqual([['setNavigationTarget', mapPosition]]);
    await app.emitNavigation(mapPosition);
    let bar = page.getByRole('link', { name: 'Target details' });
    await expect(bar).toContainText('Map position');
    await page.waitForFunction(() =>
      window.__updraftApp!.mapState.map?.getLayer('navigation-target-marker'),
    );
    expect(
      await page.evaluate(() => {
        let state = window.__updraftApp!.mapState;
        return {
          zoom: state.map!.getZoom(),
          bearing: state.map!.getBearing(),
          follow: state.followMode,
        };
      }),
    ).toEqual(camera);
    let barBounds = await bar.boundingBox();
    let mapBounds = await page.locator('.maplibregl-canvas').boundingBox();
    expect(barBounds!.y + barBounds!.height).toBeLessThanOrEqual(mapBounds!.y + 1);
    await bar.click();
    await page.getByRole('button', { name: 'Stop navigation' }).click();
    await expect(page).toHaveURL('/');
    expect((await app.navigationCommands()).at(-1)).toEqual(['setNavigationTarget', null]);
    await app.emitNavigation(null);
    await expect(bar).toBeHidden();
    await expect
      .poll(() =>
        page.evaluate(() =>
          window.__updraftApp!.mapState.map?.getLayer('navigation-target-marker'),
        ),
      )
      .toBeFalsy();
  });
}

test('selects traffic by ID and shows an unavailable suffix without a saved position', async ({
  page,
  app,
}) => {
  await app.open('/traffic/icao:ABC123');
  await page.getByRole('button', { name: 'Navigate to traffic' }).click();
  await expect(page).toHaveURL('/');
  let traffic = { type: 'traffic', id: 'icao:ABC123' } as const;
  expect(await app.navigationCommands()).toEqual([['setNavigationTarget', traffic]]);
  await app.emitNavigation(traffic);
  let bar = page.getByRole('link', { name: 'Target details' });
  await expect(bar).toContainText('icao:ABC123 (n/a)');
});

test('reports navigation changes that were not saved', async ({ page, app }) => {
  await app.open('/nearby/50.823/6.186');
  await app.queueNavigationReplies(false);
  await page.getByRole('button', { name: 'Navigate here' }).click();
  await expect(page.getByRole('alert')).toContainText(
    'A previous target can return after restart.',
  );
  await app.emitNavigation(mapPosition);
  await page.getByRole('link', { name: 'Back to map' }).click();
  await page.getByRole('link', { name: 'Target details' }).click();
  await app.queueNavigationReplies(false);
  await page.getByRole('button', { name: 'Stop navigation' }).click();
  await expect(page.getByRole('alert')).toContainText(
    'A previous target can return after restart.',
  );
  expect(await app.navigationCommands()).toEqual([
    ['setNavigationTarget', mapPosition],
    ['setNavigationTarget', null],
  ]);
});

test('selection rows open details and send pin and navigation commands', async ({ page, app }) => {
  await app.open('/navigation');
  await app.emitPins([pinnedTarget(0, home)]);
  await page.getByRole('link', { name: 'Home', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Home' })).toBeVisible();
  await page.goBack();
  await app.queueNavigationReplies(false);
  await page.getByRole('button', { name: 'Unpin target', exact: true }).click();
  await app.emitPins([]);
  await expect(page.getByRole('alert')).toContainText('not saved');
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByRole('alert')).toBeHidden();
  expect(await app.navigationCommands()).toEqual([
    ['unpinTarget', 0],
    ['unpinTarget', 0],
  ]);
  await app.emit({ topic: 'recentTargets', value: [home] });
  await expect(page.getByRole('link', { name: 'Home', exact: true })).toHaveAttribute(
    'href',
    '/navigation/recent/0',
  );
  await page.getByRole('button', { name: 'Navigate to target', exact: true }).click();
  expect((await app.navigationCommands()).at(-1)).toEqual(['setNavigationTarget', home]);
  await app.emitNavigation(home);
  await expect(page.getByRole('link', { name: 'Target details', exact: true })).toContainText(
    'Home',
  );
});

test('recent details wait for restored history and keep the selected snapshot', async ({
  page,
  app,
}) => {
  await app.open('/navigation/recent/0');
  let restored = { ...home, name: 'Restored' };
  await app.emit({ topic: 'recentTargets', value: [restored] });
  await expect(page.getByRole('heading', { name: 'Restored', exact: true })).toBeVisible();
  let traffic = { type: 'traffic', id: 'icao:ABC123' } as const;
  await app.emit({ topic: 'recentTargets', value: [traffic, restored] });
  await expect(page.getByRole('heading', { name: 'Restored', exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Navigation', exact: true }).click();
  await expect(page.getByRole('link', { name: 'icao:ABC123', exact: true })).toHaveAttribute(
    'href',
    '/traffic/icao:ABC123',
  );
});

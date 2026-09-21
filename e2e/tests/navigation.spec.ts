import { expect } from '@playwright/test';

import { test } from './app';

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
    let bar = page.getByRole('link', { name: 'Target details' });
    await expect(bar).toContainText('Map position');
    await expect
      .poll(() =>
        page.evaluate(() => {
          let app = window.__updraftApp!;
          return app.navigation.current?.target;
        }),
      )
      .toEqual({ type: 'mapPosition', latitudeDegrees: 50.823, longitudeDegrees: 6.186 });
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
  let bar = page.getByRole('link', { name: 'Target details' });
  await expect(bar).toContainText('icao:ABC123 (n/a)');
  expect(await page.evaluate(() => window.__updraftApp!.navigation.current?.target)).toEqual({
    type: 'traffic',
    id: 'icao:ABC123',
  });
  expect(await page.evaluate(() => window.__updraftApp!.navigation.current?.position)).toBeNull();
  await bar.click();
  await page.getByRole('button', { name: 'Stop navigation' }).click();
  await expect(bar).toBeHidden();
});

test('reports a save failure while navigation remains active', async ({ page, app }) => {
  await app.open('/nearby/50.823/6.186');
  await page.evaluate(() => {
    let client = window.__updraftFake!;
    let select = client.setNavigationTarget.bind(client);
    client.setNavigationTarget = async (target) => {
      await select(target);
      return false;
    };
  });
  await page.getByRole('button', { name: 'Navigate here' }).click();
  await expect(page.getByRole('alert')).toContainText(
    'A previous target can return after restart.',
  );
  expect(await page.evaluate(() => window.__updraftApp!.navigation.current?.target.type)).toBe(
    'mapPosition',
  );
  await page.getByRole('link', { name: 'Back to map' }).click();
  await page.getByRole('link', { name: 'Target details' }).click();
  await page.getByRole('button', { name: 'Stop navigation' }).click();
  await expect(page.getByRole('alert')).toContainText(
    'A previous target can return after restart.',
  );
  expect(await page.evaluate(() => window.__updraftApp!.navigation.current)).toBeNull();
});

test('selection rows open details and unpinning moves a target into recents', async ({
  page,
  app,
}) => {
  await app.open('/navigation');
  await page.evaluate(async () => {
    let client = window.__updraftFake!;
    await client.pinTarget({
      type: 'waypoint',
      name: 'Home',
      latitudeDegrees: 50,
      longitudeDegrees: 6,
      elevationMeters: 100,
    });
  });
  await page.getByRole('link', { name: 'Home', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Home' })).toBeVisible();
  expect(await page.evaluate(() => window.__updraftApp!.navigation.current)).toBeNull();
  await page.goBack();
  await page.evaluate(() => {
    let client = window.__updraftFake!;
    let unpin = client.unpinTarget.bind(client);
    let fail = true;
    client.unpinTarget = async (id) => {
      await unpin(id);
      if (fail) {
        fail = false;
        return false;
      }
      return true;
    };
  });
  await page.getByRole('button', { name: 'Unpin target', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('not saved');
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByRole('alert')).toBeHidden();
  await expect(page.getByRole('link', { name: 'Home', exact: true })).toHaveAttribute(
    'href',
    '/navigation/recent/0',
  );
  await page.getByRole('button', { name: 'Navigate to target', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Target details', exact: true })).toContainText(
    'Home',
  );
});

test('recent details wait for restored history and keep the selected snapshot', async ({
  page,
  app,
}) => {
  await app.open('/navigation/recent/0');
  await page.evaluate(async () => {
    await window.__updraftFake!.setNavigationTarget({
      type: 'waypoint',
      name: 'Restored',
      latitudeDegrees: 50,
      longitudeDegrees: 6,
      elevationMeters: 100,
    });
  });
  await expect(page.getByRole('heading', { name: 'Restored', exact: true })).toBeVisible();
  await page.evaluate(async () => {
    await window.__updraftFake!.setNavigationTarget({ type: 'traffic', id: 'icao:ABC123' });
  });
  await expect(page.getByRole('heading', { name: 'Restored', exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Navigation', exact: true }).click();
  await expect(page.getByRole('link', { name: 'icao:ABC123', exact: true })).toHaveAttribute(
    'href',
    '/traffic/icao:ABC123',
  );
});

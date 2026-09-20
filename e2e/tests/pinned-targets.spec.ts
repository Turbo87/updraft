import { expect } from '@playwright/test';

import { test } from './app';

for (let viewport of [
  { width: 390, height: 844 },
  { width: 844, height: 390 },
]) {
  test(`pins independently and hides only the primary row at ${viewport.width}x${viewport.height}`, async ({
    page,
    app,
  }) => {
    await page.setViewportSize(viewport);
    await app.open('/nearby/50.823/6.186');
    await page.getByRole('button', { name: 'Pin target', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Unpin target', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Navigate here' }).click();
    await expect(page.getByRole('region', { name: 'Pinned targets' })).toBeHidden();
    await page.getByRole('link', { name: 'Target details', exact: true }).click();
    await page.getByRole('button', { name: 'Stop navigation' }).click();
    let panel = page.getByRole('region', { name: 'Pinned targets' });
    await expect(panel.getByRole('link')).toHaveCount(1);
    await panel.getByRole('link').click();
    await expect(page.getByRole('heading', { name: 'Map position' })).toBeVisible();
    await page.getByRole('button', { name: 'Navigate to target' }).click();
    await expect(panel).toBeHidden();
    await page.getByRole('link', { name: 'Target details', exact: true }).click();
    await page.getByRole('button', { name: 'Unpin target', exact: true }).click();
    expect(await page.evaluate(() => window.__updraftApp!.navigation.current?.target.type)).toBe(
      'mapPosition',
    );
    await page.getByRole('button', { name: 'Stop navigation' }).click();
    await expect(panel).toBeHidden();
  });
}

test('retries a failed unpin without re-pinning the target', async ({ page, app }) => {
  await app.open('/traffic/icao:ABC123');
  await page.getByRole('button', { name: 'Pin target', exact: true }).click();
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
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect(page.getByRole('alert')).toBeHidden();
  await expect(page.getByRole('button', { name: 'Pin target', exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.__updraftApp!.navigation.pins)).toEqual([]);
});

for (let viewport of [
  { width: 390, height: 844 },
  { width: 844, height: 390 },
]) {
  test(`scrolls pins without covering the map at ${viewport.width}x${viewport.height}`, async ({
    page,
    app,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    await app.open('/');
    await page.evaluate(async () => {
      for (let index = 0; index < 12; index++) {
        await window.__updraftFake!.pinTarget({
          type: 'waypoint',
          name: `Field ${index + 1}`,
          latitudeDegrees: 50,
          longitudeDegrees: 6 + index / 100,
          elevationMeters: 100,
        });
      }
    });
    let panel = page.getByRole('region', { name: 'Pinned targets' });
    await expect(panel.getByRole('link')).toHaveCount(12);
    await expect(panel.getByRole('link').first()).toContainText('Field 1');
    await expect(panel.getByRole('link').last()).toContainText('Field 12');
    let bounds = await panel.boundingBox();
    let map = await page.locator('.maplibregl-canvas').boundingBox();
    expect(bounds!.height).toBeLessThanOrEqual(viewport.height * 0.3 + 1);
    expect(map!.height).toBeGreaterThan(100);
    expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(map!.y + 1);
    await panel.getByRole('link').last().scrollIntoViewIfNeeded();
    expect(await panel.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
    await page.screenshot({ path: testInfo.outputPath('pinned-targets.png') });
  });
}

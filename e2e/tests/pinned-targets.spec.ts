import { expect } from '@playwright/test';

import { pinnedTarget, test } from './app';

const mapPosition = {
  type: 'mapPosition',
  latitudeDegrees: 50.823,
  longitudeDegrees: 6.186,
} as const;

for (let viewport of [
  { width: 390, height: 844 },
  { width: 844, height: 390 },
]) {
  test(`pins a target and hides only the primary row at ${viewport.width}x${viewport.height}`, async ({
    page,
    app,
  }) => {
    await page.setViewportSize(viewport);
    await app.open('/nearby/50.823/6.186');
    await page.getByRole('button', { name: 'Pin target', exact: true }).click();
    expect(await app.navigationCommands()).toEqual([['pinTarget', mapPosition]]);
    await app.emitPins([pinnedTarget(0, mapPosition)]);
    await expect(page.getByRole('button', { name: 'Unpin target', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Navigate here' }).click();
    await app.emitNavigation(mapPosition);
    await app.emitPins([pinnedTarget(0, mapPosition, true)]);
    let panel = page.getByRole('region', { name: 'Pinned targets' });
    await expect(page.getByRole('link', { name: 'Target details', exact: true })).toBeVisible();
    await expect(panel).toBeHidden();
    await app.emitNavigation(null);
    await app.emitPins([pinnedTarget(0, mapPosition)]);
    await expect(panel.getByRole('link')).toHaveCount(1);
    await panel.getByRole('link').click();
    await expect(page.getByRole('heading', { name: 'Map position' })).toBeVisible();
    await page.getByRole('button', { name: 'Unpin target', exact: true }).click();
    expect(await app.navigationCommands()).toEqual([
      ['pinTarget', mapPosition],
      ['setNavigationTarget', mapPosition],
      ['unpinTarget', 0],
    ]);
  });
}

test('retries a failed unpin without re-pinning the target', async ({ page, app }) => {
  await app.open('/traffic/icao:ABC123');
  let traffic = { type: 'traffic', id: 'icao:ABC123' } as const;
  await app.emitPins([pinnedTarget(0, traffic)]);
  await app.queueNavigationReplies(false);
  await page.getByRole('button', { name: 'Unpin target', exact: true }).click();
  await app.emitPins([]);
  await expect(page.getByRole('alert')).toContainText('not saved');
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect(page.getByRole('alert')).toBeHidden();
  expect(await app.navigationCommands()).toEqual([
    ['unpinTarget', 0],
    ['unpinTarget', 0],
  ]);
  await expect(page.getByRole('button', { name: 'Pin target', exact: true })).toBeVisible();
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
    await app.emitPins(
      Array.from({ length: 12 }, (_, index) =>
        pinnedTarget(index, {
          type: 'waypoint',
          name: `Field ${index + 1}`,
          latitudeDegrees: 50,
          longitudeDegrees: 6 + index / 100,
          elevationMeters: 100,
        }),
      ),
    );
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

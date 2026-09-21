import { expect } from '@playwright/test';

import { waypointsFixture } from '../../frontend/src/lib/map/waypoint.fixture';
import { test } from './app';

for (let viewport of [
  { width: 390, height: 844 },
  { width: 844, height: 390 },
]) {
  test(`edits and navigates a task at ${viewport.width}x${viewport.height}`, async ({
    page,
    app,
  }) => {
    await page.setViewportSize(viewport);
    await page.addInitScript((data) => {
      window.__updraftTestWaypointData = data;
    }, waypointsFixture);
    await app.open('/task');
    await app.emit({
      topic: 'waypoints',
      value: {
        generation: 1,
        sources: [{ type: 'active', sourceName: 'local.cup', waypointCount: 3, warnings: [] }],
      },
    });
    for (let name of ['Point 0', 'Point 1', 'Point 2']) {
      await page.getByRole('button', { name, exact: true }).last().click();
    }
    let points = page.getByRole('list').getByRole('listitem');
    await expect(points).toHaveCount(3);
    await page.waitForFunction(
      () =>
        window.__updraftApp!.mapState.map?.getLayer('task-route-line') &&
        window.__updraftApp!.mapState.map?.getLayer('task-cylinder-outline'),
    );
    await points.nth(2).getByRole('button', { name: 'Move up' }).click();
    await expect(points.nth(1)).toContainText('Point 2');
    await points.nth(1).getByRole('button', { name: 'Point 2', exact: true }).click();
    await expect(page.getByText('Tracking task', { exact: true })).toBeVisible();
    await expect(points.nth(1)).toHaveAttribute('aria-current', 'step');
    await page.getByRole('button', { name: 'Pin target', exact: true }).click();
    await page.getByRole('link', { name: 'Navigation', exact: true }).click();
    await expect(page.getByRole('link', { name: 'Task', exact: true })).toHaveCount(1);
    await page.evaluate(async () => {
      await window.__updraftFake!.setNavigationTarget({ type: 'traffic', id: 'icao:ABC123' });
    });
    await page.getByRole('link', { name: 'Task', exact: true }).click();
    await expect(page.getByText('Tracking task', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Stop task' }).click();
    await expect(page.getByText('Task stopped', { exact: true })).toBeVisible();
    expect(await page.evaluate(() => window.__updraftApp!.navigation.current?.target.type)).toBe(
      'traffic',
    );
  });
}

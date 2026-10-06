import type { Task } from '$lib/protocol/generated/Task';
import type { WaypointFeature } from '$lib/waypoints';

import { expect } from '@playwright/test';

import { waypointsFixture } from '../../frontend/src/lib/map/waypoint.fixture';
import { waypointTarget } from '../../frontend/src/lib/navigation-target';
import { pinnedTarget, test } from './app';

const targets = waypointsFixture.features.map((feature) =>
  waypointTarget(feature as WaypointFeature),
);

function task(order: number[], status: Task['status'] = 'stopped', current = order[0]): Task {
  return {
    points: order.map((id) => ({ id, target: targets[id] })),
    current,
    status,
    nextId: targets.length,
    start: null,
    finish: null,
    restartAllowed: true,
  };
}

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
    await page.getByRole('button', { name: 'Point 0', exact: true }).click();
    expect(await app.navigationCommands()).toEqual([
      ['changeTask', { type: 'add', target: targets[0] }],
    ]);
    await app.emit({ topic: 'task', value: task([0, 1, 2]) });
    let points = page.getByRole('list').getByRole('listitem');
    await expect(points).toHaveCount(3);
    await page.waitForFunction(
      () =>
        window.__updraftApp!.mapState.map?.getLayer('task-route-line') &&
        window.__updraftApp!.mapState.map?.getLayer('task-cylinder-outline'),
    );
    await points.nth(2).getByRole('button', { name: 'Move up' }).click();
    await app.emit({ topic: 'task', value: task([0, 2, 1]) });
    await expect(points.nth(1)).toContainText('Point 2');
    await points.nth(1).getByRole('button', { name: 'Point 2', exact: true }).click();
    await app.emit({ topic: 'task', value: task([0, 2, 1], 'running', 2) });
    await app.emitNavigation({ type: 'task' });
    await expect(page.getByText('Tracking task', { exact: true })).toBeVisible();
    await expect(points.nth(1)).toHaveAttribute('aria-current', 'step');
    await page.getByRole('button', { name: 'Pin target', exact: true }).click();
    await app.emitPins([pinnedTarget(0, { type: 'task' }, true)]);
    await page.getByRole('link', { name: 'Navigation', exact: true }).click();
    await expect(page.getByRole('link', { name: 'Task', exact: true })).toHaveCount(1);
    await app.emitNavigation({ type: 'traffic', id: 'icao:ABC123' });
    await app.emitPins([pinnedTarget(0, { type: 'task' })]);
    await page.getByRole('link', { name: 'Task', exact: true }).click();
    await expect(page.getByText('Tracking task', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Stop task' }).click();
    await app.emit({ topic: 'task', value: task([0, 2, 1]) });
    await expect(page.getByText('Task stopped', { exact: true })).toBeVisible();
    expect((await app.navigationCommands()).slice(1)).toEqual([
      ['changeTask', { type: 'move', id: 2, index: 1 }],
      ['changeTask', { type: 'select', id: 2 }],
      ['pinTarget', { type: 'task' }],
      ['changeTask', { type: 'stop' }],
    ]);
  });
}

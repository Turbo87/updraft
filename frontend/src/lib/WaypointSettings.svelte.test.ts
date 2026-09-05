import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import WaypointSettings from './WaypointSettings.svelte';

it('imports through the fixed action bar and reports command failures', async () => {
  let onImport = vi.fn(async () => {
    throw 'parseFailed';
  });
  await render(WaypointSettings, {
    status: { generation: 0, sources: [] },
    onImport,
    onRemove: async () => {},
  });
  await expect.element(page.getByText('No waypoint files imported.')).toBeVisible();
  let button = page.getByRole('button', { name: 'Import CUP file' });
  expect(button.element().closest('footer')).not.toBeNull();
  await button.click();
  expect(onImport).toHaveBeenCalledOnce();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('The file has an invalid header or no valid waypoints.');
});

it('shows each source and its import warnings', async () => {
  await render(WaypointSettings, {
    status: {
      generation: 1,
      sources: [
        {
          type: 'active',
          sourceName: 'a.cup',
          waypointCount: 2,
          warnings: [
            { line: 4, message: 'Skipped waypoint: invalid latitude' },
            { line: 5, message: 'Ignored field: invalid runway' },
          ],
        },
        { type: 'unavailable', sourceName: 'b.cup', error: 'readFailed' },
      ],
    },
    onImport: async () => ({ type: 'cancelled' }),
    onRemove: async () => {},
  });
  await expect.element(page.getByRole('heading', { name: 'a.cup' })).toBeVisible();
  await expect.element(page.getByText('2 waypoints', { exact: true })).toBeVisible();
  await page.getByText('Import warnings (2)').click();
  await expect.element(page.getByText('Line 4: Skipped waypoint: invalid latitude')).toBeVisible();
  await expect.element(page.getByText('Could not read this stored file.')).toBeVisible();
});

it('requires confirmation before removal and reports removal failure', async () => {
  let onRemove = vi.fn(async () => {
    throw 'storageFailed';
  });
  await render(WaypointSettings, {
    status: {
      generation: 1,
      sources: [
        {
          type: 'active',
          sourceName: 'local.cup',
          waypointCount: 1,
          warnings: [],
        },
      ],
    },
    onImport: async () => ({ type: 'cancelled' }),
    onRemove,
  });
  await page.getByRole('button', { name: 'Remove file' }).click();
  expect(onRemove).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(onRemove).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Remove file' }).click();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  expect(onRemove).toHaveBeenCalledWith('local.cup');
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not update waypoint files. Please try again.');
});

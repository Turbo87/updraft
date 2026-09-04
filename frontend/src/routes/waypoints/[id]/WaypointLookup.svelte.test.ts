import type { Map } from 'maplibre-gl';

import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import { waypointsFixture } from '$lib/map/waypoint.fixture';
import WaypointLookup from './WaypointLookup.svelte';

function mapWithData(getData = async () => waypointsFixture) {
  return {
    getSource: () => ({ getData, serialize: () => ({ data: waypointsFixture }), setData: vi.fn() }),
    isSourceLoaded: () => true,
    on: vi.fn(),
    off: vi.fn(),
  } as unknown as Map;
}

it('loads the full source and rejects a link from a previous generation', async () => {
  let screen = await render(WaypointLookup, {
    map: mapWithData(),
    id: '1:0:0',
    generation: 1,
    altitudeUnit: 'm',
    sourceStatus: 'ready',
    onBack: () => {},
  });
  await expect.element(page.getByRole('heading', { name: 'Point 0' })).toBeVisible();
  await screen.rerender({ generation: 2 });
  await expect.element(page.getByText('This waypoint is no longer available.')).toBeVisible();
  await expect.element(page.getByRole('heading', { name: 'Point 0' })).not.toBeInTheDocument();
});

it('reports source failures and retries the lookup', async () => {
  let getData = vi
    .fn()
    .mockRejectedValueOnce(new Error('Unavailable'))
    .mockResolvedValue(waypointsFixture);
  await render(WaypointLookup, {
    map: mapWithData(getData),
    id: '1:0:0',
    generation: 1,
    altitudeUnit: 'm',
    sourceStatus: 'ready',
    onBack: () => {},
  });
  await expect.element(page.getByText('Could not load waypoints.')).toBeVisible();
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect.element(page.getByRole('heading', { name: 'Point 0' })).toBeVisible();
});

it('shows a source error that occurred before the details opened', async () => {
  let getData = vi.fn(async () => waypointsFixture);
  let screen = await render(WaypointLookup, {
    map: mapWithData(getData),
    id: '1:0:0',
    generation: 1,
    altitudeUnit: 'm',
    sourceStatus: 'failed',
    onBack: () => {},
  });
  await expect.element(page.getByText('Could not load waypoints.')).toBeVisible();
  expect(getData).not.toHaveBeenCalled();
  await screen.rerender({ sourceStatus: 'ready' });
  await expect.element(page.getByRole('heading', { name: 'Point 0' })).toBeVisible();
});

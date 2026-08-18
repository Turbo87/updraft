import { createRawSnippet } from 'svelte';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import NearbyResultsScreen from './NearbyResultsScreen.svelte';

const airspaces = createRawSnippet(() => ({ render: () => '<p>Airspace results</p>' }));
const traffic = createRawSnippet(() => ({ render: () => '<p>Traffic results</p>' }));

const props = {
  airspaces,
  backLabel: 'Back to map',
  ownshipRelation: {
    distance: { value: '4.2', unit: 'km' },
    bearing: { value: '063', unit: '°' },
  },
  position: { latitudeDegrees: 50.82341, longitudeDegrees: 6.18604 },
  summary: {
    arrivalHeight: { stale: true, value: '—' },
    requiredGlideRatio: { stale: true, value: '—' },
    terrainElevation: { stale: true, value: '—' },
  },
  title: 'Nearby',
  traffic,
};

describe('NearbyResultsScreen.svelte', () => {
  it('shows every summary value and both result sections', async () => {
    render(NearbyResultsScreen, props);

    await expect.element(page.getByRole('heading', { level: 1, name: 'Nearby' })).toBeVisible();
    await expect
      .element(page.getByRole('link', { name: 'Back to map' }))
      .toHaveAttribute('href', '/');
    await expect.element(page.getByText('50.82341° N, 6.18604° E')).toBeVisible();
    await expect.element(page.getByText('Distance')).toBeVisible();
    await expect.element(page.getByText('Bearing')).toBeVisible();
    await expect.element(page.getByText('Arrival')).toBeVisible();
    await expect.element(page.getByText('Req. L/D')).toBeVisible();
    await expect.element(page.getByText('Elevation')).toBeVisible();
    await expect.element(page.getByRole('heading', { level: 2, name: 'Airspaces' })).toBeVisible();
    await expect.element(page.getByText('Airspace results')).toBeVisible();
    await expect.element(page.getByRole('heading', { level: 2, name: 'Traffic' })).toBeVisible();
    await expect.element(page.getByText('Traffic results')).toBeVisible();
  });

  it('explains when ownship-dependent values are unavailable', async () => {
    render(NearbyResultsScreen, {
      ...props,
      ownshipRelation: null,
      position: { latitudeDegrees: -50.79118, longitudeDegrees: -6.44052 },
    });

    await expect.element(page.getByText('50.79118° S, 6.44052° W')).toBeVisible();
    await expect
      .element(
        page.getByText('No GPS position is available. Values relative to ownship are unknown.'),
      )
      .toBeVisible();
  });
});

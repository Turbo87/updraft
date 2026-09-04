import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import WaypointDetails from './WaypointDetails.svelte';

const properties = {
  id: '1:0:0',
  sourceName: 'local.cup',
  name: 'Field',
  kind: 2,
  elevationMeters: 304.8,
  runwayDirection: 90,
  runwayLengthMeters: 800,
  frequency: '123.500',
  notes: 'First line\nSecond line',
};

it('shows source data and converts elevation to the selected unit', async () => {
  await render(WaypointDetails, {
    waypoint: { type: 'Feature', geometry: { type: 'Point', coordinates: [6, 50] }, properties },
    altitudeUnit: 'ft',
    onBack: () => {},
  });
  await expect.element(page.getByRole('heading', { name: 'Field' })).toBeVisible();
  await expect.element(page.getByText('Grass airfield')).toBeVisible();
  await expect.element(page.getByText('1000 ft MSL')).toBeVisible();
  await expect.element(page.getByText('090°')).toBeVisible();
  await expect.element(page.getByText('800 m')).toBeVisible();
  await expect.element(page.getByText('123.500')).toBeVisible();
  await expect.element(page.getByText('local.cup')).toBeVisible();
  await expect.element(page.getByText('First line Second line')).toBeVisible();
});

it('omits runway, radio, and notes when the source does not supply them', async () => {
  await render(WaypointDetails, {
    waypoint: {
      type: 'Feature',
      geometry: { type: 'Point', coordinates: [6, 50] },
      properties: {
        id: '1:0:0',
        sourceName: 'a.cup',
        name: 'Peak',
        kind: 7,
        elevationMeters: 100,
        frequency: '',
        notes: '',
      },
    },
    altitudeUnit: 'm',
    onBack: () => {},
  });
  await expect.element(page.getByText('Mountain top')).toBeVisible();
  await expect.element(page.getByText('100 m MSL')).toBeVisible();
  await expect.element(page.getByText('Runway direction')).not.toBeInTheDocument();
  await expect.element(page.getByText('Frequency')).not.toBeInTheDocument();
});

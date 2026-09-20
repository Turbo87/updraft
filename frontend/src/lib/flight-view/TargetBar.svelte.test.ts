import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import { defaultSettings } from '$lib/settings';
import TargetBar from './TargetBar.svelte';

it('shows relative bearing and distance, then true bearing when track is unavailable', async () => {
  let navigation = {
    target: {
      type: 'waypoint' as const,
      name: 'Home',
      latitudeDegrees: 50,
      longitudeDegrees: 6,
      elevationMeters: 100,
    },
    position: { latitudeDegrees: 50, longitudeDegrees: 6 },
    traffic: null,
    arrival: { marginMeters: 250, stale: false },
    guidance: {
      distanceMeters: 12300,
      bearingDegrees: 90,
      relativeBearingDegrees: -15,
      stale: false,
    },
  };
  let screen = await render(TargetBar, { navigation, units: defaultSettings().units });
  await expect
    .element(page.getByRole('link', { name: 'Target details' }))
    .toHaveAttribute('href', '/navigation');
  await expect.element(page.getByText('Home', { exact: true })).toBeVisible();
  await expect.element(page.getByLabelText('Arrival margin')).toHaveTextContent('+250 m');
  await expect.element(page.getByLabelText('Relative bearing')).toHaveTextContent('◁ 15°');
  await expect.element(page.getByText('12.3 km')).toBeVisible();
  await screen.rerender({
    navigation: {
      ...navigation,
      guidance: { ...navigation.guidance, relativeBearingDegrees: null, stale: true },
    },
  });
  await expect.element(page.getByLabelText('True bearing')).toHaveTextContent('90° T');
  await expect.element(page.getByRole('link', { name: 'Target details' })).toHaveClass('stale');
});

it('shows waiting traffic without guidance, then the retained report age and relative altitude', async () => {
  let navigation = {
    target: { type: 'traffic' as const, id: 'icao:ABC123' },
    position: null,
    guidance: null,
    arrival: null,
    traffic: null,
  };
  let screen = await render(TargetBar, { navigation, units: defaultSettings().units });
  await expect.element(page.getByText('Waiting for traffic')).toBeVisible();
  await expect.element(page.getByLabelText('Relative altitude')).toHaveTextContent('–');
  await expect.element(page.getByLabelText('Arrival margin')).not.toBeInTheDocument();
  await screen.rerender({
    navigation: {
      ...navigation,
      position: { latitudeDegrees: 50, longitudeDegrees: 6 },
      guidance: {
        bearingDegrees: 90,
        relativeBearingDegrees: null,
        distanceMeters: 1000,
        stale: true,
      },
      traffic: {
        name: 'ABC',
        ageSeconds: 35,
        stale: true,
        relativeAltitude: { meters: -50, stale: true },
      },
    },
  });
  await expect.element(page.getByText('Waiting for traffic')).not.toBeInTheDocument();
  await expect.element(page.getByText('Last report 35s ago')).toBeVisible();
  await expect.element(page.getByLabelText('Relative altitude')).toHaveTextContent('-50 m');
  await expect.element(page.getByLabelText('Relative altitude')).toHaveClass('stale');
});

import type { PinnedTarget } from '$lib/protocol/generated/PinnedTarget';

import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import { settingsFixture } from '$lib/settings.fixture';
import PinnedTargets from './PinnedTargets.svelte';

it('displays pin guidance with units and updates missing and stale values', async () => {
  let pin: PinnedTarget = {
    id: 5,
    primary: false,
    navigation: {
      target: {
        type: 'waypoint',
        name: 'Home',
        latitudeDegrees: 50,
        longitudeDegrees: 6,
        elevationMeters: 100,
      },
      position: { latitudeDegrees: 50, longitudeDegrees: 6 },
      guidance: {
        distanceMeters: 1852,
        bearingDegrees: 90,
        relativeBearingDegrees: -7,
        stale: false,
      },
      arrival: { marginMeters: 304.8, stale: false },
      traffic: null,
    },
  };
  let units = settingsFixture({
    units: { altitude: 'ft', distance: 'nm', speed: 'kt', verticalSpeed: 'ft/min' },
  }).units;
  let screen = await render(PinnedTargets, { pins: [pin], units, hasPrimary: false });
  await expect.element(page.getByRole('link')).toHaveAttribute('href', '/pinned-targets/5');
  await expect.element(page.getByLabelText('Arrival margin')).toHaveTextContent('+1,000 ft');
  await expect.element(page.getByText('1 nm', { exact: true })).toBeVisible();
  await expect.element(page.getByLabelText('Relative bearing')).toHaveTextContent('◁ 7°');
  await screen.rerender({
    pins: [
      {
        ...pin,
        navigation: {
          ...pin.navigation,
          arrival: { marginMeters: 304.8, stale: true },
          guidance: null,
        },
      },
    ],
  });
  await expect.element(page.getByLabelText('Arrival margin')).toHaveClass('stale');
  await expect.element(page.getByLabelText('True bearing')).toHaveTextContent('–');
  await screen.rerender({
    pins: [
      {
        ...pin,
        navigation: {
          ...pin.navigation,
          target: { type: 'traffic', id: 'icao:ABC123' },
          arrival: null,
          traffic: {
            name: 'ABC',
            ageSeconds: 31,
            stale: true,
            relativeAltitude: { meters: -304.8, stale: true },
          },
        },
      },
    ],
  });
  await expect.element(page.getByLabelText('Arrival margin')).not.toBeInTheDocument();
  await expect.element(page.getByLabelText('Relative altitude')).toHaveTextContent('-1,000 ft');
  await expect.element(page.getByText('Last report 31s ago')).toBeVisible();
});

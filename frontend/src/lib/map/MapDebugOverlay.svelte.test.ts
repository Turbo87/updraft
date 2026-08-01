import type { Instruments } from '$lib/protocol/generated/Instruments';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import MapDebugOverlay from './MapDebugOverlay.svelte';

const emptyInstruments: Instruments = {
  position: null,
  altitudeMslMeters: null,
  trackDegrees: null,
  groundSpeedMetersPerSecond: null,
};

describe('MapDebugOverlay.svelte', () => {
  it('is hidden until the D key is pressed, then toggles off again', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments });

    let zoom = page.getByText('Zoom');
    await expect.element(zoom).not.toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).not.toBeInTheDocument();
  });

  it('offers a tile-boundaries checkbox once visible', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments });

    await userEvent.keyboard('d');

    let checkbox = page.getByRole('checkbox', { name: 'Tile boundaries' });
    await expect.element(checkbox).not.toBeChecked();
    await checkbox.click();
    await expect.element(checkbox).toBeChecked();
  });

  it('shows the current flight values once visible', async () => {
    let instruments: Instruments = {
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      altitudeMslMeters: 190,
      trackDegrees: 45,
      groundSpeedMetersPerSecond: 30,
    };
    render(MapDebugOverlay, { map: undefined, instruments });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('50.82300, 6.18600')).toBeInTheDocument();
    await expect.element(page.getByText('190 m', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('108.0 km/h', { exact: true })).toBeInTheDocument();
  });
});

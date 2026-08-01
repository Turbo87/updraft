import type { Instruments } from '$lib/protocol/generated/Instruments';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

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

const metricUnits: UnitSettings = {
  altitude: 'm',
  distance: 'km',
  speed: 'km/h',
  verticalSpeed: 'm/s',
};

describe('MapDebugOverlay.svelte', () => {
  it('is hidden until the D key is pressed, then toggles off again', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments, units: metricUnits });

    let zoom = page.getByText('Zoom');
    await expect.element(zoom).not.toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).not.toBeInTheDocument();
  });

  it('offers a tile-boundaries checkbox once visible', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments, units: metricUnits });

    await userEvent.keyboard('d');

    let checkbox = page.getByRole('checkbox', { name: 'Tile boundaries' });
    await expect.element(checkbox).not.toBeChecked();
    await checkbox.click();
    await expect.element(checkbox).toBeChecked();
  });

  it('keeps missing values and map coordinates unchanged', async () => {
    let view = await render(MapDebugOverlay, {
      map: undefined,
      instruments: emptyInstruments,
      units: metricUnits,
    });

    await userEvent.keyboard('d');

    let values = Array.from(view.container.querySelectorAll('dd'), (element) =>
      element.textContent?.trim(),
    );
    expect(values).toEqual(['0.00', '0.00000, 0.00000', '–', '–', '–']);
  });

  it('shows the current flight values once visible', async () => {
    let instruments: Instruments = {
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      altitudeMslMeters: 190,
      trackDegrees: 45,
      groundSpeedMetersPerSecond: 30,
    };
    render(MapDebugOverlay, { map: undefined, instruments, units: metricUnits });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('50.82300, 6.18600')).toBeInTheDocument();
    await expect.element(page.getByText('190 m', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('108.0 km/h', { exact: true })).toBeInTheDocument();
  });

  it('uses the selected altitude and speed units', async () => {
    let instruments: Instruments = {
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      altitudeMslMeters: 190,
      trackDegrees: 45,
      groundSpeedMetersPerSecond: 30,
    };
    let units: UnitSettings = {
      altitude: 'ft',
      distance: 'nm',
      speed: 'kt',
      verticalSpeed: 'ft/min',
    };
    render(MapDebugOverlay, { map: undefined, instruments, units });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('50.82300, 6.18600')).toBeInTheDocument();
    await expect.element(page.getByText('623 ft', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('58.3 kt', { exact: true })).toBeInTheDocument();
  });
});

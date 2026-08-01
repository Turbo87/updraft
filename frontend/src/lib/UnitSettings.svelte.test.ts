import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import UnitSettings from './UnitSettings.svelte';

describe('UnitSettings.svelte', () => {
  it('shows one labeled select for each unit selection', async () => {
    render(UnitSettings, {
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      onUnitsChange: async () => {},
    });

    await expect
      .element(page.getByRole('combobox', { name: 'Altitude', exact: true }))
      .toHaveValue('m');
    await expect
      .element(page.getByRole('combobox', { name: 'Distance', exact: true }))
      .toHaveValue('km');
    await expect
      .element(page.getByRole('combobox', { name: 'Speed', exact: true }))
      .toHaveValue('km/h');
    await expect
      .element(page.getByRole('combobox', { name: 'Vertical speed', exact: true }))
      .toHaveValue('m/s');
  });

  it('reports one complete value when a selection changes', async () => {
    let onUnitsChange = vi.fn(async () => {});
    render(UnitSettings, {
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      onUnitsChange,
    });

    await page.getByRole('combobox', { name: 'Altitude', exact: true }).selectOptions('ft');

    expect(onUnitsChange).toHaveBeenCalledExactlyOnceWith({
      altitude: 'ft',
      distance: 'km',
      speed: 'km/h',
      verticalSpeed: 'm/s',
    });
  });

  it('reports each optimistic value without waiting for an earlier update', async () => {
    let releaseFirst = () => {};
    let firstCall = new Promise<void>((resolve) => {
      releaseFirst = resolve;
    });
    let onUnitsChange = vi.fn(async () => {});
    onUnitsChange.mockReturnValueOnce(firstCall);
    let view = await render(UnitSettings, {
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      onUnitsChange,
    });

    await page.getByRole('combobox', { name: 'Altitude', exact: true }).selectOptions('ft');
    await view.rerender({
      units: { altitude: 'ft', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      onUnitsChange,
    });
    await page.getByRole('combobox', { name: 'Distance', exact: true }).selectOptions('nm');

    try {
      expect(onUnitsChange).toHaveBeenNthCalledWith(1, {
        altitude: 'ft',
        distance: 'km',
        speed: 'km/h',
        verticalSpeed: 'm/s',
      });
      expect(onUnitsChange).toHaveBeenNthCalledWith(2, {
        altitude: 'ft',
        distance: 'nm',
        speed: 'km/h',
        verticalSpeed: 'm/s',
      });
    } finally {
      releaseFirst();
    }
  });
});

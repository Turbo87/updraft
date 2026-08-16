import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import UnitSettings from './UnitSettings.svelte';

describe('UnitSettings.svelte', () => {
  it('shows one radio group for each unit selection', async () => {
    render(UnitSettings, {
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      onUnitsChange: async () => {},
    });

    await expect.element(page.getByRole('group', { name: 'Altitude' })).toBeVisible();
    await expect.element(page.getByRole('group', { name: 'Distance' })).toBeVisible();
    await expect.element(page.getByRole('group', { name: 'Speed', exact: true })).toBeVisible();
    await expect.element(page.getByRole('group', { name: 'Vertical speed' })).toBeVisible();
    await expect.element(page.getByRole('radio', { name: 'm', exact: true })).toBeChecked();
    await expect.element(page.getByRole('radio', { name: 'km', exact: true })).toBeChecked();
    await expect.element(page.getByRole('radio', { name: 'km/h', exact: true })).toBeChecked();
    await expect.element(page.getByRole('radio', { name: 'm/s', exact: true })).toBeChecked();
    await expect.element(page.getByRole('combobox')).not.toBeInTheDocument();
  });

  it('reports one complete value when a selection changes', async () => {
    let onUnitsChange = vi.fn(async () => {});
    render(UnitSettings, {
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      onUnitsChange,
    });

    await page.getByText('ft', { exact: true }).click();

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

    await page.getByText('ft', { exact: true }).click();
    await view.rerender({
      units: { altitude: 'ft', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      onUnitsChange,
    });
    await page.getByText('nm', { exact: true }).click();

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

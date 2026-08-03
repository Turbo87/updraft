import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import DevicesScreen from './DevicesScreen.svelte';

describe('DevicesScreen.svelte', () => {
  it('waits for the first external-device topic before showing the list state', async () => {
    render(DevicesScreen, {
      devices: [],
      initialized: false,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange: async () => {},
    });

    await expect.element(page.getByText('Loading external devices…')).toBeInTheDocument();
    await expect.element(page.getByText('No external devices configured.')).not.toBeInTheDocument();
  });

  it('shows an empty state without creating a default device', async () => {
    render(DevicesScreen, {
      devices: [],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange: async () => {},
    });

    await expect
      .element(page.getByRole('heading', { name: 'External devices' }))
      .toBeInTheDocument();
    await expect.element(page.getByText('No external devices configured.')).toBeInTheDocument();
  });

  it('links back to the Settings screen', async () => {
    render(DevicesScreen, {
      devices: [],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange: async () => {},
    });

    await expect
      .element(page.getByRole('link', { name: 'Back to settings' }))
      .toHaveAttribute('href', '/settings');
  });

  it('links to device creation and configured-device editing', async () => {
    render(DevicesScreen, {
      devices: [
        { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
        {
          deviceId: 5,
          enabled: true,
          type: 'bluetooth',
          address: '00:11:22:33:44:55',
        },
      ],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange: async () => {},
    });

    await expect
      .element(page.getByRole('link', { name: 'Add external device' }))
      .toHaveAttribute('href', '/settings/devices/new');
    await expect
      .element(page.getByRole('link', { name: 'Edit 192.0.2.1:4353' }))
      .toHaveAttribute('href', '/settings/devices/4');
    await expect
      .element(page.getByRole('link', { name: 'Edit 00:11:22:33:44:55' }))
      .toHaveAttribute('href', '/settings/devices/5');
  });

  it('summarizes TCP and Bluetooth devices without showing the standard SPP UUID', async () => {
    render(DevicesScreen, {
      devices: [
        { deviceId: 1, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
        {
          deviceId: 2,
          enabled: true,
          type: 'bluetooth',
          address: '00:11:22:33:44:55',
        },
        {
          deviceId: 3,
          enabled: false,
          type: 'bluetooth',
          address: 'AA:BB:CC:DD:EE:FF',
          serviceUuid: '12345678-1234-1234-1234-123456789abc',
        },
      ],
      initialized: true,
      bondedBluetoothDevices: {
        status: 'available',
        devices: [{ address: '00:11:22:33:44:55', name: 'Flight recorder' }],
      },
      onEnabledChange: async () => {},
    });

    await expect.element(page.getByText('TCP', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('192.0.2.1:4353', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('Flight recorder', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('00:11:22:33:44:55', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('AA:BB:CC:DD:EE:FF', { exact: true })).toBeInTheDocument();
    await expect
      .element(page.getByText('12345678-1234-1234-1234-123456789abc', { exact: true }))
      .toBeInTheDocument();
    await expect
      .element(page.getByText('00001101-0000-1000-8000-00805F9B34FB', { exact: true }))
      .not.toBeInTheDocument();
  });

  it('shows the published enabled state for each device', async () => {
    render(DevicesScreen, {
      devices: [
        { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
        {
          deviceId: 7,
          enabled: false,
          type: 'bluetooth',
          address: '00:11:22:33:44:55',
        },
      ],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange: async () => {},
    });

    let rows = page.getByRole('listitem');
    await expect.element(rows.nth(0).getByRole('switch', { name: 'Enabled' })).toBeChecked();
    await expect.element(rows.nth(1).getByRole('switch', { name: 'Enabled' })).not.toBeChecked();
  });

  it('requests an enabled change and waits for a published device update', async () => {
    let onEnabledChange = vi.fn(async () => {});
    let device = {
      deviceId: 4,
      enabled: true,
      type: 'tcp' as const,
      host: '192.0.2.1',
      port: 4353,
    };
    let view = await render(DevicesScreen, {
      devices: [device],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange,
    });

    let enabledSwitch = page.getByRole('switch', { name: 'Enabled' });
    await enabledSwitch.click();

    expect(onEnabledChange).toHaveBeenCalledExactlyOnceWith(4, false);
    await expect.element(enabledSwitch).toBeChecked();

    await view.rerender({
      devices: [{ ...device, enabled: false }],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange,
    });
    await expect.element(enabledSwitch).not.toBeChecked();

    await enabledSwitch.click();
    expect(onEnabledChange).toHaveBeenNthCalledWith(2, 4, true);
  });

  it('disables only the switch whose command is pending', async () => {
    let finishChange = () => {};
    let pendingChange = new Promise<void>((resolve) => {
      finishChange = resolve;
    });
    let onEnabledChange = vi.fn(() => pendingChange);
    render(DevicesScreen, {
      devices: [
        { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
        { deviceId: 7, enabled: false, type: 'tcp', host: '192.0.2.2', port: 4353 },
      ],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange,
    });

    let switches = page.getByRole('switch', { name: 'Enabled' });
    await switches.nth(0).click();

    await expect.element(switches.nth(0)).toBeDisabled();
    await expect.element(switches.nth(1)).toBeEnabled();

    finishChange();
    await expect.element(switches.nth(0)).toBeEnabled();
  });

  it('restores the published state and shows an error when a command fails', async () => {
    let onEnabledChange = vi.fn(async () => {
      throw new Error('driver stopped');
    });
    render(DevicesScreen, {
      devices: [{ deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 }],
      initialized: true,
      bondedBluetoothDevices: { status: 'unsupported' },
      onEnabledChange,
    });

    let enabledSwitch = page.getByRole('switch', { name: 'Enabled' });
    await enabledSwitch.click();

    await expect.element(enabledSwitch).toBeChecked();
    await expect.element(enabledSwitch).toBeEnabled();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not update this device.');
  });
});

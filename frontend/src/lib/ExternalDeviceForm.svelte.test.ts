import type { ComponentProps } from 'svelte';

import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import ExternalDeviceForm from './ExternalDeviceForm.svelte';

type ExternalDeviceFormProps = ComponentProps<typeof ExternalDeviceForm>;
type RenderExternalDeviceFormProps = Omit<ExternalDeviceFormProps, 'getBondedBluetoothDevices'> &
  Partial<Pick<ExternalDeviceFormProps, 'getBondedBluetoothDevices'>>;

function renderExternalDeviceForm({
  getBondedBluetoothDevices = async () => ({ status: 'unsupported' }),
  ...props
}: RenderExternalDeviceFormProps): void {
  render(ExternalDeviceForm, { getBondedBluetoothDevices, ...props });
}

describe('ExternalDeviceForm.svelte', () => {
  it('owns the add-device screen navigation', async () => {
    renderExternalDeviceForm({ onSave: async () => {} });

    await expect
      .element(page.getByRole('heading', { name: 'Add external device' }))
      .toBeInTheDocument();
    await expect
      .element(page.getByRole('link', { name: 'Back to external devices' }))
      .toHaveAttribute('href', '/settings/devices');
  });

  it('creates a bonded Bluetooth device with the standard service', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({
      getBondedBluetoothDevices: async () => ({
        status: 'available',
        devices: [{ address: '00:11:22:33:44:55', name: 'Flight recorder' }],
      }),
      onSave,
    });

    await page.getByLabelText('Connection type').selectOptions('bluetooth');
    await page.getByRole('radio', { name: 'Flight recorder 00:11:22:33:44:55' }).click();
    await page.getByRole('button', { name: 'Add external device' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
    });
    await expect
      .element(page.getByText('00001101-0000-1000-8000-00805F9B34FB', { exact: true }))
      .not.toBeInTheDocument();
  });

  it('refreshes the bonded devices after permission is granted', async () => {
    let getBondedBluetoothDevices = vi
      .fn()
      .mockResolvedValueOnce({ status: 'permissionDenied' })
      .mockResolvedValueOnce({
        status: 'available',
        devices: [{ address: '00:11:22:33:44:55', name: null }],
      });
    renderExternalDeviceForm({ getBondedBluetoothDevices, onSave: async () => {} });

    await page.getByLabelText('Connection type').selectOptions('bluetooth');
    await expect
      .element(page.getByText('Allow Nearby Devices access to select a Bluetooth device.'))
      .toBeInTheDocument();

    await page.getByRole('button', { name: 'Refresh bonded devices' }).click();

    await expect
      .element(page.getByRole('radio', { name: '00:11:22:33:44:55' }))
      .toBeInTheDocument();
    expect(getBondedBluetoothDevices).toHaveBeenCalledTimes(2);
  });

  it('distinguishes disabled Bluetooth from denied permission', async () => {
    renderExternalDeviceForm({
      getBondedBluetoothDevices: async () => ({ status: 'disabled' }),
      onSave: async () => {},
    });

    await page.getByLabelText('Connection type').selectOptions('bluetooth');

    await expect
      .element(page.getByText('Turn on Bluetooth to select a bonded device.'))
      .toBeInTheDocument();
    await expect
      .element(page.getByText('Allow Nearby Devices access to select a Bluetooth device.'))
      .not.toBeInTheDocument();
  });

  it('preserves a saved Bluetooth address while Bluetooth is disabled', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({
      device: {
        deviceId: 4,
        enabled: true,
        type: 'bluetooth',
        address: '00:11:22:33:44:55',
      },
      getBondedBluetoothDevices: async () => ({ status: 'disabled' }),
      onSave,
    });

    await expect
      .element(page.getByText('Turn on Bluetooth to select a bonded device.'))
      .toBeInTheDocument();
    await expect.element(page.getByText('00:11:22:33:44:55', { exact: true })).toBeInTheDocument();
    await page.getByRole('button', { name: 'Save changes' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
    });
  });

  it('directs an Android user with no bonded devices to system settings', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({
      getBondedBluetoothDevices: async () => ({ status: 'available', devices: [] }),
      onSave,
    });

    await page.getByLabelText('Connection type').selectOptions('bluetooth');

    await expect
      .element(
        page.getByText('Pair a Bluetooth device in Android settings, then refresh this list.'),
      )
      .toBeInTheDocument();
    await page.getByRole('button', { name: 'Add external device' }).click();
    expect(onSave).not.toHaveBeenCalled();
  });

  it('preserves a custom service UUID while replacing an unbonded address', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({
      device: {
        deviceId: 4,
        enabled: true,
        type: 'bluetooth',
        address: '00:11:22:33:44:55',
        serviceUuid: '12345678-1234-1234-1234-123456789abc',
      },
      getBondedBluetoothDevices: async () => ({
        status: 'available',
        devices: [{ address: 'AA:BB:CC:DD:EE:FF', name: 'New flight recorder' }],
      }),
      onSave,
    });

    await expect
      .element(page.getByRole('radio', { name: '00:11:22:33:44:55 (not currently bonded)' }))
      .toBeInTheDocument();
    await expect
      .element(page.getByText('12345678-1234-1234-1234-123456789abc', { exact: true }))
      .toBeInTheDocument();

    await page.getByRole('radio', { name: 'New flight recorder AA:BB:CC:DD:EE:FF' }).click();
    await page.getByRole('button', { name: 'Save changes' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'bluetooth',
      address: 'AA:BB:CC:DD:EE:FF',
      serviceUuid: '12345678-1234-1234-1234-123456789abc',
    });
  });

  it('changes a Bluetooth device to TCP on Android', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({
      device: {
        deviceId: 4,
        enabled: true,
        type: 'bluetooth',
        address: '00:11:22:33:44:55',
      },
      getBondedBluetoothDevices: async () => ({
        status: 'available',
        devices: [{ address: '00:11:22:33:44:55', name: null }],
      }),
      onSave,
    });

    await page.getByLabelText('Connection type').selectOptions('tcp');
    await page.getByLabelText('Host').fill('flight-recorder.local');
    await page.getByLabelText('Port').fill('4353');
    await page.getByRole('button', { name: 'Save changes' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'tcp',
      host: 'flight-recorder.local',
      port: 4353,
    });
  });

  it('keeps a saved Bluetooth device read-only on desktop', async () => {
    renderExternalDeviceForm({
      device: {
        deviceId: 4,
        enabled: true,
        type: 'bluetooth',
        address: '00:11:22:33:44:55',
        serviceUuid: '12345678-1234-1234-1234-123456789abc',
      },
      getBondedBluetoothDevices: async () => ({ status: 'unsupported' }),
      onSave: async () => {},
      onDelete: async () => {},
    });

    await expect.element(page.getByLabelText('Connection type')).toBeDisabled();
    await expect.element(page.getByText('00:11:22:33:44:55', { exact: true })).toBeInTheDocument();
    await expect
      .element(page.getByText('12345678-1234-1234-1234-123456789abc', { exact: true }))
      .toBeInTheDocument();
    await expect
      .element(page.getByRole('button', { name: 'Save changes' }))
      .not.toBeInTheDocument();
    await expect
      .element(page.getByRole('button', { name: 'Delete external device' }))
      .toBeInTheDocument();
  });

  it('shows a bonded-device query error and permits a retry', async () => {
    let getBondedBluetoothDevices = vi
      .fn()
      .mockRejectedValueOnce(new Error('plugin unavailable'))
      .mockResolvedValueOnce({
        status: 'available',
        devices: [{ address: '00:11:22:33:44:55', name: null }],
      });
    renderExternalDeviceForm({ getBondedBluetoothDevices, onSave: async () => {} });

    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not load bonded Bluetooth devices.');
    await page.getByRole('button', { name: 'Refresh bonded devices' }).click();

    await expect.element(page.getByRole('option', { name: 'Bluetooth SPP' })).toBeInTheDocument();
    expect(getBondedBluetoothDevices).toHaveBeenCalledTimes(2);
  });

  it('requires one bonded device before saving Bluetooth', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({
      getBondedBluetoothDevices: async () => ({
        status: 'available',
        devices: [{ address: '00:11:22:33:44:55', name: null }],
      }),
      onSave,
    });

    await page.getByLabelText('Connection type').selectOptions('bluetooth');
    await page.getByRole('button', { name: 'Add external device' }).click();

    await expect.element(page.getByText('Select a bonded Bluetooth device.')).toBeInTheDocument();
    expect(onSave).not.toHaveBeenCalled();
  });

  it('creates a TCP device with a trimmed host and numeric port', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({ onSave });

    await page.getByLabelText('Host').fill('  flight-recorder.local  ');
    await page.getByLabelText('Port').fill('4353');
    await page.getByRole('button', { name: 'Add external device' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'tcp',
      host: 'flight-recorder.local',
      port: 4353,
    });
  });

  it('identifies invalid TCP fields without saving', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({ onSave });

    await page.getByLabelText('Host').fill('   ');
    await page.getByLabelText('Port').fill('65536');
    await page.getByRole('button', { name: 'Add external device' }).click();

    await expect.element(page.getByText('Enter a host.')).toBeInTheDocument();
    await expect
      .element(page.getByText('Enter a whole-number port from 1 to 65535.'))
      .toBeInTheDocument();
    expect(onSave).not.toHaveBeenCalled();
  });

  it('rejects ports outside the decimal integer range', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({ onSave });
    await page.getByLabelText('Host').fill('192.0.2.1');

    for (let port of ['0', '65536', '1.5', 'not-a-port']) {
      await page.getByLabelText('Port').fill(port);
      await page.getByRole('button', { name: 'Add external device' }).click();
      await expect
        .element(page.getByText('Enter a whole-number port from 1 to 65535.'))
        .toBeInTheDocument();
    }

    expect(onSave).not.toHaveBeenCalled();
  });

  it('prefills and edits a TCP device', async () => {
    let onSave = vi.fn(async () => {});
    renderExternalDeviceForm({
      device: { deviceId: 4, enabled: false, type: 'tcp', host: '192.0.2.1', port: 4353 },
      onSave,
    });

    await expect.element(page.getByLabelText('Host')).toHaveValue('192.0.2.1');
    await expect.element(page.getByLabelText('Port')).toHaveValue('4353');

    await page.getByLabelText('Host').fill('192.0.2.2');
    await page.getByLabelText('Port').fill('10110');
    await page.getByRole('button', { name: 'Save changes' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'tcp',
      host: '192.0.2.2',
      port: 10110,
    });
  });

  it('disables Save while the command is pending', async () => {
    let finishSave = () => {};
    let pendingSave = new Promise<void>((resolve) => {
      finishSave = resolve;
    });
    renderExternalDeviceForm({ onSave: () => pendingSave });

    await page.getByLabelText('Host').fill('192.0.2.1');
    await page.getByLabelText('Port').fill('4353');
    let saveButton = page.getByRole('button', { name: 'Add external device' });
    await saveButton.click();

    await expect.element(saveButton).toBeDisabled();
    finishSave();
    await expect.element(saveButton).toBeEnabled();
  });

  it('keeps the form open and shows an error when Save fails', async () => {
    renderExternalDeviceForm({
      onSave: async () => {
        throw new Error('driver stopped');
      },
    });

    await page.getByLabelText('Host').fill('192.0.2.1');
    await page.getByLabelText('Port').fill('4353');
    let saveButton = page.getByRole('button', { name: 'Add external device' });
    await saveButton.click();

    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not save this external device.');
    await expect.element(page.getByLabelText('Host')).toHaveValue('192.0.2.1');
    await expect.element(saveButton).toBeEnabled();
  });

  it('deletes an existing TCP device only after confirmation', async () => {
    let onDelete = vi.fn(async () => {});
    renderExternalDeviceForm({
      device: { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
      onSave: async () => {},
      onDelete,
    });

    await page.getByRole('button', { name: 'Delete external device' }).click();

    await expect
      .element(page.getByRole('dialog', { name: 'Delete 192.0.2.1:4353?' }))
      .toBeInTheDocument();
    expect(onDelete).not.toHaveBeenCalled();

    await page.getByRole('button', { name: 'Delete', exact: true }).click();
    expect(onDelete).toHaveBeenCalledOnce();
  });

  it('keeps the confirmation open and shows an error when Delete fails', async () => {
    renderExternalDeviceForm({
      device: { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
      onSave: async () => {},
      onDelete: async () => {
        throw new Error('driver stopped');
      },
    });

    await page.getByRole('button', { name: 'Delete external device' }).click();
    await page.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect
      .element(page.getByRole('dialog', { name: 'Delete 192.0.2.1:4353?' }))
      .toBeInTheDocument();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not delete this external device.');
  });
});

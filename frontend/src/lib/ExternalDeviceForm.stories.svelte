<script module lang="ts">
  import type { BondedBluetoothDevices } from '$lib/client/bonded-bluetooth-devices';
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import { m } from '$lib/paraglide/messages.js';
  import ExternalDeviceForm from './ExternalDeviceForm.svelte';

  const tcpDevice = {
    deviceId: 4,
    enabled: true,
    type: 'tcp',
    host: '192.0.2.1',
    port: 4353,
  } satisfies PublishedExternalDevice;

  const bluetoothDevice = {
    deviceId: 5,
    enabled: true,
    type: 'bluetooth',
    address: '00:11:22:33:44:55',
  } satisfies PublishedExternalDevice;

  async function getAvailableBondedBluetoothDevices(): Promise<BondedBluetoothDevices> {
    return {
      status: 'available',
      devices: [
        { address: '00:11:22:33:44:55', name: 'Flight recorder' },
        { address: 'AA:BB:CC:DD:EE:FF', name: null },
      ],
    };
  }

  async function getPermissionDenied(): Promise<BondedBluetoothDevices> {
    return { status: 'permissionDenied' };
  }

  async function getDisabledBluetooth(): Promise<BondedBluetoothDevices> {
    return { status: 'disabled' };
  }

  async function getNoBondedBluetoothDevices(): Promise<BondedBluetoothDevices> {
    return { status: 'available', devices: [] };
  }

  async function getUnsupportedBondedBluetoothDevices(): Promise<BondedBluetoothDevices> {
    return { status: 'unsupported' };
  }

  const { Story } = defineMeta({
    title: 'Components/ExternalDeviceForm',
    component: ExternalDeviceForm,
    args: {
      getBondedBluetoothDevices: getUnsupportedBondedBluetoothDevices,
      onSave: fn(async () => {}),
    },
    parameters: {
      docs: {
        description: {
          component:
            'Use this form to add or edit one TCP or Bluetooth SPP connection. TCP requires a non-empty host and a whole-number port from 1 to 65535. Android can select a bonded Bluetooth device, refresh the bonded-device list, and distinguish denied permission, disabled Bluetooth, and an empty list. Desktop keeps existing Bluetooth settings read-only because Bluetooth discovery is unavailable. Preserve a custom service UUID when an existing device uses one. Saving and deletion keep the form open after an error so the user can retry.',
        },
        story: { autoplay: true },
      },
    },
  });
</script>

<Story name="Add TCP" />

<Story
  name="TCP validation errors"
  play={async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole('button', { name: m.add_external_device() }));
  }}
/>

<Story
  name="Save pending"
  args={{ device: tcpDevice, onSave: fn(() => new Promise<void>(() => {})) }}
  play={async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole('button', { name: m.save_external_device() }));
  }}
/>

<Story
  name="Save error"
  args={{
    device: tcpDevice,
    onSave: fn(async () => {
      throw new Error('Device command rejected');
    }),
  }}
  play={async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole('button', { name: m.save_external_device() }));
  }}
/>

<Story name="Edit TCP" args={{ device: tcpDevice, onDelete: fn(async () => {}) }} />

<Story
  name="Delete confirmation"
  args={{ device: tcpDevice, onDelete: fn(async () => {}) }}
  parameters={{ docs: { story: { autoplay: false } } }}
  play={async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole('button', { name: m.delete_external_device() }));
  }}
/>

<Story
  name="Edit Bluetooth"
  args={{
    device: bluetoothDevice,
    getBondedBluetoothDevices: getAvailableBondedBluetoothDevices,
    onDelete: fn(async () => {}),
  }}
/>

<Story
  name="Custom service UUID"
  args={{
    device: {
      deviceId: 5,
      enabled: true,
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
      serviceUuid: '12345678-1234-1234-1234-123456789abc',
    },
    getBondedBluetoothDevices: getAvailableBondedBluetoothDevices,
    onDelete: fn(async () => {}),
  }}
/>

<Story
  name="Unbonded Bluetooth"
  args={{
    device: {
      deviceId: 5,
      enabled: true,
      type: 'bluetooth',
      address: '12:34:56:78:9A:BC',
    },
    getBondedBluetoothDevices: getAvailableBondedBluetoothDevices,
    onDelete: fn(async () => {}),
  }}
/>

<Story
  name="Bluetooth permission denied"
  args={{
    device: bluetoothDevice,
    getBondedBluetoothDevices: getPermissionDenied,
    onDelete: fn(async () => {}),
  }}
/>

<Story
  name="Bluetooth disabled"
  args={{
    device: bluetoothDevice,
    getBondedBluetoothDevices: getDisabledBluetooth,
    onDelete: fn(async () => {}),
  }}
/>

<Story
  name="No bonded Bluetooth devices"
  args={{ getBondedBluetoothDevices: getNoBondedBluetoothDevices }}
  play={async ({ canvas, userEvent }) => {
    await userEvent.selectOptions(canvas.getByLabelText(m.connection_type()), 'bluetooth');
  }}
/>

<Story
  name="Bonded-device query error"
  args={{
    getBondedBluetoothDevices: async () => {
      throw new Error('Bluetooth plugin unavailable');
    },
  }}
/>

<Story
  name="Desktop Bluetooth"
  args={{
    device: bluetoothDevice,
    getBondedBluetoothDevices: getUnsupportedBondedBluetoothDevices,
    onDelete: fn(async () => {}),
  }}
/>

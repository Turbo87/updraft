<script module lang="ts">
  import type { BondedBluetoothDevices } from '$lib/client/bonded-bluetooth-devices';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ExternalDeviceForm from './ExternalDeviceForm.svelte';

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

  async function getUnsupportedBondedBluetoothDevices(): Promise<BondedBluetoothDevices> {
    return { status: 'unsupported' };
  }

  const { Story } = defineMeta({
    title: 'Components/ExternalDeviceForm',
    component: ExternalDeviceForm,
  });
</script>

<Story
  name="Add TCP"
  args={{ getBondedBluetoothDevices: getUnsupportedBondedBluetoothDevices, onSave: async () => {} }}
/>

<Story
  name="Edit Bluetooth"
  args={{
    device: {
      deviceId: 5,
      enabled: true,
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
    },
    getBondedBluetoothDevices: getAvailableBondedBluetoothDevices,
    onSave: async () => {},
    onDelete: async () => {},
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
    onSave: async () => {},
    onDelete: async () => {},
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
    onSave: async () => {},
    onDelete: async () => {},
  }}
/>

<Story
  name="Bluetooth permission denied"
  args={{
    device: {
      deviceId: 5,
      enabled: true,
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
    },
    getBondedBluetoothDevices: getPermissionDenied,
    onSave: async () => {},
    onDelete: async () => {},
  }}
/>

<Story
  name="Desktop Bluetooth"
  args={{
    device: {
      deviceId: 5,
      enabled: true,
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
    },
    getBondedBluetoothDevices: getUnsupportedBondedBluetoothDevices,
    onSave: async () => {},
    onDelete: async () => {},
  }}
/>

<Story
  name="Edit TCP"
  args={{
    device: { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
    getBondedBluetoothDevices: getUnsupportedBondedBluetoothDevices,
    onSave: async () => {},
    onDelete: async () => {},
  }}
/>

<Story
  name="Command error"
  args={{
    getBondedBluetoothDevices: getUnsupportedBondedBluetoothDevices,
    onSave: async () => {
      throw new Error('Device command rejected');
    },
  }}
/>

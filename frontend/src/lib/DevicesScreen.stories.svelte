<script module lang="ts">
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import DevicesScreen from './DevicesScreen.svelte';

  const mixedDevices = [
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
  ] satisfies PublishedExternalDevice[];

  const { Story } = defineMeta({
    title: 'Screens/Devices',
    component: DevicesScreen,
  });
</script>

<Story
  name="Empty"
  args={{
    devices: [],
    initialized: true,
    bondedBluetoothDevices: { status: 'unsupported' },
    onEnabledChange: async () => {},
  }}
/>

<Story
  name="Mixed devices"
  args={{
    devices: mixedDevices,
    initialized: true,
    bondedBluetoothDevices: {
      status: 'available',
      devices: [{ address: '00:11:22:33:44:55', name: 'Flight recorder' }],
    },
    onEnabledChange: async () => {},
  }}
/>

<Story
  name="Command error"
  args={{
    devices: [mixedDevices[0]],
    initialized: true,
    bondedBluetoothDevices: { status: 'unsupported' },
    onEnabledChange: async () => {
      throw new Error('Device command rejected');
    },
  }}
/>

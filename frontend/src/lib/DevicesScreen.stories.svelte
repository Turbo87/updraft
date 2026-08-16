<script module lang="ts">
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

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
    args: {
      bondedBluetoothDevices: { status: 'unsupported' },
      devices: [],
      initialized: true,
      onEnabledChange: fn(async () => {}),
    },
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Use this screen to list and manage external data sources. Keep loading and empty states distinct. A configured device shows its connection type, endpoint, enabled state, and edit action. Show the bonded Bluetooth name when the platform provides one. Keep the Bluetooth address visible for identification. Show a custom service UUID only when the device uses one. An enabled-state update disables only the affected device until the command finishes. A failed update keeps the previous state and displays an error.',
        },
      },
    },
  });
</script>

<Story name="Loading" args={{ initialized: false }} />

<Story name="Empty" />

<Story
  name="Configured devices"
  args={{
    devices: mixedDevices,
    bondedBluetoothDevices: {
      status: 'available',
      devices: [{ address: '00:11:22:33:44:55', name: 'Flight recorder' }],
    },
  }}
/>

<Story
  name="Update pending"
  args={{
    devices: [mixedDevices[0]],
    onEnabledChange: fn(() => new Promise<void>(() => {})),
  }}
  parameters={{
    docs: { description: { story: 'Change the switch to show the pending state.' } },
  }}
/>

<Story
  name="Update error"
  args={{
    devices: [mixedDevices[0]],
    onEnabledChange: fn(async () => {
      throw new Error('Device command rejected');
    }),
  }}
  parameters={{
    docs: { description: { story: 'Change the switch to show the failed update state.' } },
  }}
/>

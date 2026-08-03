<script lang="ts">
  import type { BondedBluetoothDevices } from '$lib/client/bonded-bluetooth-devices';

  import { onMount } from 'svelte';

  import { getAppContext } from '$lib/app-context';
  import DevicesScreen from '$lib/DevicesScreen.svelte';

  const { client, externalDevices } = getAppContext();
  let bondedBluetoothDevices = $state.raw<BondedBluetoothDevices>({ status: 'unsupported' });

  onMount(() => {
    let active = true;
    void client
      .getBondedBluetoothDevices()
      .then((result) => {
        if (active) bondedBluetoothDevices = result;
      })
      .catch((error: unknown) => {
        console.error('Failed to query bonded Bluetooth devices', error);
      });

    return () => {
      active = false;
    };
  });
</script>

<DevicesScreen
  devices={externalDevices.current}
  initialized={externalDevices.initialized}
  {bondedBluetoothDevices}
  onEnabledChange={(deviceId, enabled) => client.setExternalDeviceEnabled(deviceId, enabled)}
/>

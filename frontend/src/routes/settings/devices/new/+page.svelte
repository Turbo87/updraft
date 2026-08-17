<script lang="ts">
  import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import { getAppContext } from '$lib/app-context';
  import ExternalDeviceForm from '$lib/ExternalDeviceForm.svelte';

  const { client } = getAppContext();

  async function addExternalDevice(spec: ConnectionSpec): Promise<void> {
    await client.addExternalDevice(spec);
    await goto(resolve('/settings/devices'));
  }
</script>

<ExternalDeviceForm
  getBondedBluetoothDevices={() => client.getBondedBluetoothDevices()}
  onSave={addExternalDevice}
/>

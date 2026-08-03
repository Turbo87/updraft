<script lang="ts">
  import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import { getAppContext } from '$lib/app-context';
  import ExternalDeviceForm from '$lib/ExternalDeviceForm.svelte';
  import { m } from '$lib/paraglide/messages.js';

  const { client } = getAppContext();

  async function addExternalDevice(spec: ConnectionSpec): Promise<void> {
    await client.addExternalDevice(spec);
    await goto(resolve('/settings/devices'));
  }
</script>

<main>
  <a class="back-link" href={resolve('/settings/devices')}>{m.back_to_external_devices()}</a>
  <h1>{m.add_external_device()}</h1>
  <ExternalDeviceForm
    getBondedBluetoothDevices={() => client.getBondedBluetoothDevices()}
    onSave={addExternalDevice}
  />
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  .back-link {
    display: inline-block;
    margin-block-end: 1rem;
  }

  h1 {
    margin-block-start: 0;
  }
</style>

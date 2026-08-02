<script lang="ts">
  import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import ExternalDeviceForm from '$lib/ExternalDeviceForm.svelte';
  import { m } from '$lib/paraglide/messages.js';

  const { client, externalDevices } = getAppContext();
  const deviceId = $derived.by(() => {
    let routeDeviceId = page.params.deviceId;
    return routeDeviceId !== undefined && /^\d+$/.test(routeDeviceId)
      ? Number(routeDeviceId)
      : undefined;
  });
  const device = $derived(
    deviceId === undefined
      ? undefined
      : externalDevices.current.find((candidate) => candidate.deviceId === deviceId),
  );
  let commandDeviceNotFound = $state(false);

  function isUnknownDeviceError(error: unknown): boolean {
    if (typeof error !== 'object' || error === null) return false;
    if (!('kind' in error) || !('deviceId' in error)) return false;
    return error.kind === 'unknownExternalDevice' && error.deviceId === deviceId;
  }

  async function editExternalDevice(spec: ConnectionSpec): Promise<void> {
    if (deviceId === undefined) return;

    try {
      await client.editExternalDevice(deviceId, spec);
    } catch (error) {
      if (!isUnknownDeviceError(error)) throw error;
      commandDeviceNotFound = true;
      return;
    }
    await goto(resolve('/devices'));
  }

  async function deleteExternalDevice(): Promise<void> {
    if (deviceId === undefined) return;

    try {
      await client.deleteExternalDevice(deviceId);
    } catch (error) {
      if (!isUnknownDeviceError(error)) throw error;
      commandDeviceNotFound = true;
      return;
    }
    await goto(resolve('/devices'));
  }
</script>

<main>
  {#if !externalDevices.initialized}
    <p>{m.loading_external_devices()}</p>
  {:else if !device || commandDeviceNotFound}
    <h1>{m.external_device_not_found()}</h1>
    <a href={resolve('/devices')}>{m.back_to_external_devices()}</a>
  {:else}
    <h1>{m.edit_external_device_heading()}</h1>
    <ExternalDeviceForm
      {device}
      getBondedBluetoothDevices={() => client.getBondedBluetoothDevices()}
      onSave={editExternalDevice}
      onDelete={deleteExternalDevice}
    />
    <a href={resolve('/devices')}>{m.back_to_external_devices()}</a>
  {/if}
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  h1,
  p {
    margin: 0;
  }

  a {
    display: inline-block;
    margin-block-start: 2rem;
  }
</style>

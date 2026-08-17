<script lang="ts">
  import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import ExternalDeviceForm from '$lib/ExternalDeviceForm.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

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
    await goto(resolve('/settings/devices'));
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
    await goto(resolve('/settings/devices'));
  }
</script>

{#if !externalDevices.initialized}
  <ScreenScaffold
    backHref="/settings/devices"
    backLabel={m.back_to_external_devices()}
    title={m.external_devices_heading()}
  >
    <p>{m.loading_external_devices()}</p>
  </ScreenScaffold>
{:else if !device || commandDeviceNotFound}
  <ScreenScaffold
    backHref="/settings/devices"
    backLabel={m.back_to_external_devices()}
    title={m.external_devices_heading()}
  >
    <p>{m.external_device_not_found()}</p>
  </ScreenScaffold>
{:else}
  <ExternalDeviceForm
    {device}
    getBondedBluetoothDevices={() => client.getBondedBluetoothDevices()}
    onSave={editExternalDevice}
    onDelete={deleteExternalDevice}
  />
{/if}

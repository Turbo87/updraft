<script lang="ts">
  import type { BondedBluetoothDevices } from '$lib/client/bonded-bluetooth-devices';
  import type { ExternalDeviceId } from '$lib/protocol/generated/ExternalDeviceId';
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { resolve } from '$app/paths';

  import { m } from '$lib/paraglide/messages.js';

  type DevicesScreenProps = {
    devices: readonly PublishedExternalDevice[];
    initialized: boolean;
    bondedBluetoothDevices: BondedBluetoothDevices;
    onEnabledChange: (deviceId: ExternalDeviceId, enabled: boolean) => Promise<void>;
  };

  let { devices, initialized, bondedBluetoothDevices, onEnabledChange }: DevicesScreenProps =
    $props();
  let pendingDeviceIds = $state.raw<ExternalDeviceId[]>([]);
  let failedDeviceIds = $state.raw<ExternalDeviceId[]>([]);

  function bondedBluetoothName(device: PublishedExternalDevice): string | undefined {
    if (device.type !== 'bluetooth' || bondedBluetoothDevices.status !== 'available') {
      return undefined;
    }

    let address = device.address.toLowerCase();
    return (
      bondedBluetoothDevices.devices.find(
        (bondedDevice) => bondedDevice.address.toLowerCase() === address,
      )?.name ?? undefined
    );
  }

  function deviceEndpoint(device: PublishedExternalDevice): string {
    return device.type === 'tcp' ? `${device.host}:${device.port}` : device.address;
  }

  async function requestEnabledChange(
    event: Event & { currentTarget: HTMLInputElement },
    device: PublishedExternalDevice,
  ): Promise<void> {
    let enabled = event.currentTarget.checked;
    event.currentTarget.checked = device.enabled;
    failedDeviceIds = failedDeviceIds.filter((deviceId) => deviceId !== device.deviceId);
    pendingDeviceIds = [...pendingDeviceIds, device.deviceId];
    try {
      await onEnabledChange(device.deviceId, enabled);
    } catch {
      failedDeviceIds = [...failedDeviceIds, device.deviceId];
    } finally {
      pendingDeviceIds = pendingDeviceIds.filter((deviceId) => deviceId !== device.deviceId);
    }
  }
</script>

<main>
  <h1>{m.external_devices_heading()}</h1>

  {#if !initialized}
    <p>{m.loading_external_devices()}</p>
  {:else if devices.length === 0}
    <p>{m.no_external_devices_configured()}</p>
  {:else}
    <ul>
      {#each devices as device (device.deviceId)}
        {let bondedName = bondedBluetoothName(device)}
        <li>
          <h2>
            {device.type === 'tcp' ? m.tcp_device_type() : m.bluetooth_spp_device_type()}
          </h2>
          {#if device.type === 'tcp'}
            <p class="endpoint">{device.host}:{device.port}</p>
          {:else}
            {#if bondedName}
              <p class="endpoint">{bondedName}</p>
            {/if}
            <p class={['address', bondedName && 'secondary']}>{device.address}</p>
            {#if device.serviceUuid}
              <p class="service-uuid">
                <span>{m.custom_service_uuid()}</span>
                <code>{device.serviceUuid}</code>
              </p>
            {/if}
          {/if}
          <label>
            <input
              type="checkbox"
              role="switch"
              checked={device.enabled}
              disabled={pendingDeviceIds.includes(device.deviceId)}
              onchange={(event) => void requestEnabledChange(event, device)}
            />
            <span>{m.device_enabled()}</span>
          </label>
          <a href={resolve('/devices/[deviceId]', { deviceId: String(device.deviceId) })}
            >{m.edit_external_device({ endpoint: deviceEndpoint(device) })}</a
          >
          {#if failedDeviceIds.includes(device.deviceId)}
            <p class="error" role="alert">{m.update_device_error()}</p>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <a class="add-link" href={resolve('/devices/new')}>{m.add_external_device()}</a>
  <a class="back-link" href={resolve('/settings')}>{m.back_to_settings()}</a>
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h2 {
    font-size: 1rem;
  }

  ul {
    display: grid;
    max-width: 40rem;
    margin: 1.5rem 0 0;
    padding: 0;
    gap: 0.75rem;
    list-style: none;
  }

  li {
    display: grid;
    padding: 1rem;
    border: 0.0625rem solid light-dark(var(--color-gray-300), var(--color-gray-700));
    border-radius: 0.5rem;
    gap: 0.5rem;
  }

  .endpoint {
    font-size: 1.125rem;
    font-weight: 600;
  }

  .address,
  code {
    overflow-wrap: anywhere;
  }

  .secondary,
  .service-uuid {
    color: light-dark(var(--color-gray-600), var(--color-gray-300));
  }

  .service-uuid {
    display: grid;
    gap: 0.125rem;
    font-size: 0.875rem;
  }

  label {
    display: flex;
    min-height: 2.75rem;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  input {
    width: 1.25rem;
    height: 1.25rem;
    margin: 0;
    accent-color: var(--color-link);
  }

  input:disabled {
    cursor: wait;
  }

  .error {
    color: light-dark(var(--color-red-700), var(--color-red-300));
  }

  .add-link,
  .back-link {
    display: inline-block;
    margin-block-start: 2rem;
  }

  .back-link {
    margin-inline-start: 1rem;
  }
</style>

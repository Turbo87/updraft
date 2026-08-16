<script lang="ts">
  import type { BondedBluetoothDevices } from '$lib/client/bonded-bluetooth-devices';
  import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { onMount } from 'svelte';

  import { m } from '$lib/paraglide/messages.js';
  import RadioList from './RadioList.svelte';
  import TextField from './TextField.svelte';

  type ExternalDeviceFormProps = {
    device?: PublishedExternalDevice;
    getBondedBluetoothDevices: () => Promise<BondedBluetoothDevices>;
    onSave: (spec: ConnectionSpec) => Promise<void>;
    onDelete?: () => Promise<void>;
  };

  let { device, getBondedBluetoothDevices, onSave, onDelete }: ExternalDeviceFormProps = $props();
  let bondedBluetoothDevices = $state.raw<BondedBluetoothDevices>({ status: 'unsupported' });
  let connectionType = $derived(device?.type ?? 'tcp');
  let host = $derived(device?.type === 'tcp' ? device.host : '');
  let port = $derived(device?.type === 'tcp' ? String(device.port) : '');
  let bluetoothAddress = $derived(device?.type === 'bluetooth' ? device.address : '');
  let bluetoothQueryPending = $state(false);
  let bluetoothQueryFailed = $state(false);
  let submitted = $state(false);
  let pending = $state(false);
  let commandFailed = $state(false);
  let confirmingDelete = $state(false);
  let deletePending = $state(false);
  let deleteFailed = $state(false);
  const trimmedHost = $derived(host.trim());
  const numericPort = $derived(Number(port));
  const validPort = $derived(/^\d+$/.test(port) && numericPort >= 1 && numericPort <= 65535);
  const bluetoothSupported = $derived(bondedBluetoothDevices.status !== 'unsupported');
  const currentBluetoothAddressUnbonded = $derived.by(() => {
    if (device?.type !== 'bluetooth' || bondedBluetoothDevices.status !== 'available') {
      return false;
    }

    let currentAddress = device.address.toLowerCase();
    return !bondedBluetoothDevices.devices.some(
      (bondedDevice) => bondedDevice.address.toLowerCase() === currentAddress,
    );
  });
  let active = false;

  async function refreshBondedBluetoothDevices(): Promise<void> {
    if (bluetoothQueryPending) return;

    bluetoothQueryFailed = false;
    bluetoothQueryPending = true;
    try {
      let result = await getBondedBluetoothDevices();
      if (active) bondedBluetoothDevices = result;
    } catch {
      if (active) bluetoothQueryFailed = true;
    } finally {
      if (active) bluetoothQueryPending = false;
    }
  }

  onMount(() => {
    active = true;
    void refreshBondedBluetoothDevices();
    return () => {
      active = false;
    };
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    submitted = true;
    if (
      pending ||
      (connectionType === 'tcp' && (!trimmedHost || !validPort)) ||
      (connectionType === 'bluetooth' && !bluetoothAddress)
    ) {
      return;
    }

    commandFailed = false;
    pending = true;
    try {
      await onSave(
        connectionType === 'tcp'
          ? { type: 'tcp', host: trimmedHost, port: numericPort }
          : {
              type: 'bluetooth',
              address: bluetoothAddress,
              ...(device?.type === 'bluetooth' &&
                device.serviceUuid && { serviceUuid: device.serviceUuid }),
            },
      );
    } catch {
      commandFailed = true;
    } finally {
      pending = false;
    }
  }

  async function deleteExternalDevice(): Promise<void> {
    if (!onDelete || deletePending) return;

    deleteFailed = false;
    deletePending = true;
    try {
      await onDelete();
      confirmingDelete = false;
    } catch {
      deleteFailed = true;
    } finally {
      deletePending = false;
    }
  }

  function openDeleteConfirmation(): void {
    deleteFailed = false;
    confirmingDelete = true;
  }

  function visibleEndpoint(device: PublishedExternalDevice): string {
    return device.type === 'tcp' ? `${device.host}:${device.port}` : device.address;
  }

  function bondedDeviceOptions(): Array<{
    value: string;
    label: string;
    description?: string;
  }> {
    if (bondedBluetoothDevices.status !== 'available') return [];

    let options = bondedBluetoothDevices.devices.map((bondedDevice) => ({
      value: bondedDevice.address,
      label: bondedDevice.name ?? bondedDevice.address,
      ...(bondedDevice.name && { description: bondedDevice.address }),
    }));

    if (currentBluetoothAddressUnbonded && device?.type === 'bluetooth') {
      options.unshift({
        value: device.address,
        label: m.bluetooth_device_not_bonded({ address: device.address }),
      });
    }

    return options;
  }
</script>

<form onsubmit={(event) => void submit(event)}>
  <label class="connection-type">
    <span>{m.connection_type()}</span>
    <span class="select-wrapper">
      <select disabled={!bluetoothSupported} bind:value={connectionType}>
        {#if bluetoothSupported}
          <option value="tcp">{m.tcp_device_type()}</option>
          <option value="bluetooth">{m.bluetooth_spp_device_type()}</option>
        {:else if connectionType === 'bluetooth'}
          <option value="bluetooth">{m.bluetooth_spp_device_type()}</option>
        {:else}
          <option value="tcp">{m.tcp_device_type()}</option>
        {/if}
      </select>
      <span aria-hidden="true" class="i-mdi-chevron-down select-icon"></span>
    </span>
  </label>
  {#if connectionType === 'tcp'}
    <TextField
      name="host"
      label={m.tcp_host()}
      error={submitted && !trimmedHost ? m.tcp_host_error() : undefined}
      bind:value={host}
    />
    <TextField
      name="port"
      inputmode="numeric"
      label={m.tcp_port()}
      error={submitted && !validPort ? m.tcp_port_error() : undefined}
      bind:value={port}
    />
  {:else if bondedBluetoothDevices.status === 'available'}
    {#if bondedBluetoothDevices.devices.length === 0 && !currentBluetoothAddressUnbonded}
      <p>{m.no_bonded_bluetooth_devices()}</p>
    {:else}
      <RadioList
        name="bonded-device"
        legend={m.bonded_bluetooth_device()}
        options={bondedDeviceOptions()}
        value={bluetoothAddress}
        error={submitted && !bluetoothAddress ? m.bonded_bluetooth_device_error() : undefined}
        onChange={(value) => (bluetoothAddress = value)}
      />
    {/if}
  {:else if bondedBluetoothDevices.status === 'permissionDenied'}
    <p>{m.bluetooth_permission_denied()}</p>
  {:else if bondedBluetoothDevices.status === 'disabled'}
    <p>{m.bluetooth_disabled()}</p>
  {/if}
  {#if connectionType === 'bluetooth' && device?.type === 'bluetooth' && bondedBluetoothDevices.status !== 'available'}
    <p class="endpoint">{device.address}</p>
  {/if}
  {#if connectionType === 'bluetooth' && bluetoothSupported && !bluetoothQueryFailed}
    <button
      type="button"
      disabled={bluetoothQueryPending}
      onclick={() => void refreshBondedBluetoothDevices()}
      >{m.refresh_bonded_bluetooth_devices()}</button
    >
  {/if}
  {#if bluetoothQueryFailed}
    <p class="error" role="alert">{m.bonded_bluetooth_devices_error()}</p>
    <button
      type="button"
      disabled={bluetoothQueryPending}
      onclick={() => void refreshBondedBluetoothDevices()}
      >{m.refresh_bonded_bluetooth_devices()}</button
    >
  {/if}
  {#if connectionType === 'bluetooth' && device?.type === 'bluetooth' && device.serviceUuid}
    <p class="service-uuid">
      <span>{m.custom_service_uuid()}</span>
      <code>{device.serviceUuid}</code>
    </p>
  {/if}
  {#if connectionType === 'tcp' || bluetoothSupported}
    <button type="submit" disabled={pending}
      >{device ? m.save_external_device() : m.add_external_device()}</button
    >
  {/if}
  {#if commandFailed}
    <p class="error" role="alert">{m.save_external_device_error()}</p>
  {/if}
  {#if device && onDelete}
    <button type="button" onclick={openDeleteConfirmation}>{m.delete_external_device()}</button>
  {/if}
</form>

{#if confirmingDelete && device}
  <dialog open aria-labelledby="delete-heading">
    <h2 id="delete-heading">
      {m.confirm_delete_external_device({ endpoint: visibleEndpoint(device) })}
    </h2>
    {#if deleteFailed}
      <p class="error" role="alert">{m.delete_external_device_error()}</p>
    {/if}
    <div class="dialog-actions">
      <button type="button" disabled={deletePending} onclick={() => (confirmingDelete = false)}
        >{m.cancel()}</button
      >
      <button type="button" disabled={deletePending} onclick={() => void deleteExternalDevice()}
        >{m.confirm_delete()}</button
      >
    </div>
  </dialog>
{/if}

<style>
  form {
    display: grid;
    max-width: 30rem;
    gap: 1rem;
  }

  label {
    display: grid;
    gap: 0.25rem;
  }

  .connection-type {
    gap: var(--space-2);
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .select-wrapper {
    position: relative;
  }

  .connection-type select {
    box-sizing: border-box;
    width: 100%;
    height: var(--target-min);
    padding: 0 3rem 0 0.875rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-control);
    appearance: none;
    background: var(--color-card-surface);
    color: var(--color-text);
    font: var(--text-row-label);
    cursor: pointer;
  }

  .connection-type select:focus-visible {
    border-color: var(--color-focus-ring);
    box-shadow: inset 0 0 0 1px var(--color-focus-ring);
  }

  .connection-type select:disabled {
    border-color: var(--color-border);
    background: var(--color-disabled-surface);
    color: var(--color-disabled-text);
    cursor: not-allowed;
  }

  .select-icon {
    position: absolute;
    inset-block-start: 50%;
    inset-inline-end: 0.875rem;
    color: var(--color-text-muted);
    font-size: 1.5rem;
    pointer-events: none;
    transform: translateY(-50%);
  }

  .error {
    margin: 0;
    color: light-dark(var(--color-red-700), var(--color-red-300));
  }

  .service-uuid {
    display: grid;
    gap: 0.125rem;
  }

  .endpoint {
    font-size: 1.125rem;
    font-weight: 600;
  }

  select,
  button {
    min-height: 2.75rem;
    font: inherit;
  }

  dialog {
    max-width: calc(100% - 3rem);
    padding: 1.5rem;
    border: 0.0625rem solid light-dark(var(--color-gray-300), var(--color-gray-700));
    border-radius: 0.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  dialog h2 {
    margin-block-start: 0;
  }

  .dialog-actions {
    display: flex;
    justify-content: end;
    gap: 0.75rem;
  }
</style>

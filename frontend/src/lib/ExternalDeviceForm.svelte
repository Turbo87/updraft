<script lang="ts">
  import type { BondedBluetoothDevices } from '$lib/client/bonded-bluetooth-devices';
  import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { onMount } from 'svelte';

  import { m } from '$lib/paraglide/messages.js';
  import Button from './Button.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import RadioList from './RadioList.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';
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
  const canSave = $derived(connectionType === 'tcp' || bluetoothSupported);
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

{#snippet actions()}
  <Button
    form="external-device-form"
    loading={pending}
    size="large"
    style="width: 100%"
    type="submit"
  >
    {device ? m.save_external_device() : m.add_external_device()}
  </Button>
{/snippet}

<ScreenScaffold
  actions={canSave ? actions : undefined}
  backHref="/settings/devices"
  backLabel={m.back_to_external_devices()}
  title={device ? m.edit_external_device_heading() : m.add_external_device()}
>
  <form id="external-device-form" onsubmit={(event) => void submit(event)}>
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
      <Button
        loading={bluetoothQueryPending}
        style="width: 100%"
        variant="secondary"
        onclick={() => void refreshBondedBluetoothDevices()}
      >
        {#if !bluetoothQueryPending}
          <span aria-hidden="true" class="i-mdi-refresh action-icon"></span>
        {/if}
        {m.refresh_bonded_bluetooth_devices()}
      </Button>
    {/if}
    {#if bluetoothQueryFailed}
      <p class="error" role="alert">{m.bonded_bluetooth_devices_error()}</p>
      <Button
        loading={bluetoothQueryPending}
        style="width: 100%"
        variant="secondary"
        onclick={() => void refreshBondedBluetoothDevices()}
      >
        {#if !bluetoothQueryPending}
          <span aria-hidden="true" class="i-mdi-refresh action-icon"></span>
        {/if}
        {m.refresh_bonded_bluetooth_devices()}
      </Button>
    {/if}
    {#if connectionType === 'bluetooth' && device?.type === 'bluetooth' && device.serviceUuid}
      <p class="service-uuid">
        <span>{m.custom_service_uuid()}</span>
        <code>{device.serviceUuid}</code>
      </p>
    {/if}
    {#if commandFailed}
      <p class="error" role="alert">{m.save_external_device_error()}</p>
    {/if}
    {#if device && onDelete}
      <div class="delete-device">
        <Button
          disabled={deletePending}
          style="width: 100%"
          variant="destructive-outline"
          onclick={openDeleteConfirmation}
        >
          <span aria-hidden="true" class="i-mdi-delete-outline action-icon"></span>
          {m.delete_external_device()}
        </Button>
      </div>
    {/if}
  </form>
</ScreenScaffold>

{#if device && onDelete}
  <ConfirmDialog
    bind:open={confirmingDelete}
    title={m.confirm_delete_external_device({ endpoint: visibleEndpoint(device) })}
    description={m.delete_external_device_description()}
    cancelLabel={m.cancel()}
    confirmLabel={m.confirm_delete()}
    pending={deletePending}
    error={deleteFailed ? m.delete_external_device_error() : undefined}
    onCancel={() => (confirmingDelete = false)}
    onConfirm={() => void deleteExternalDevice()}
  />
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

  select {
    min-height: 2.75rem;
    font: inherit;
  }

  .delete-device {
    margin-block-start: var(--space-2);
    padding-block-start: var(--space-6);
    border-block-start: 1px solid var(--color-separator);
  }

  .action-icon {
    font-size: 1.5rem;
  }
</style>

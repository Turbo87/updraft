<script lang="ts">
  import type { BondedBluetoothDevices } from '$lib/client/bonded-bluetooth-devices';
  import type { ExternalDeviceId } from '$lib/protocol/generated/ExternalDeviceId';
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { resolve } from '$app/paths';

  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import StatusPill from './StatusPill.svelte';

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

{#snippet actions()}
  {#if initialized}
    <a class="add-action" href={resolve('/settings/devices/new')}>
      <span aria-hidden="true" class="i-mdi-plus"></span>
      {m.add_external_device()}
    </a>
  {:else}
    <span class="add-action disabled" aria-disabled="true">
      <span aria-hidden="true" class="i-mdi-plus"></span>
      {m.add_external_device()}
    </span>
  {/if}
{/snippet}

<ScreenScaffold
  {actions}
  backHref="/settings"
  backLabel={m.back_to_settings()}
  title={m.external_devices_heading()}
>
  {#if !initialized}
    <div class="loading-state">
      <p class="loading-label">
        <span aria-hidden="true" class="i-mdi-loading loading-icon"></span>
        {m.loading_external_devices()}
      </p>
      <div aria-hidden="true" class="skeletons">
        <div class="skeleton-card"><span></span><span class="skeleton-endpoint"></span></div>
        <div class="skeleton-card"><span></span><span class="skeleton-endpoint"></span></div>
      </div>
    </div>
  {:else if devices.length === 0}
    <div class="empty-state">
      <span aria-hidden="true" class="i-mdi-lan-disconnect empty-icon"></span>
      <h2>{m.no_external_devices_configured()}</h2>
      <p>{m.external_devices_empty_description()}</p>
    </div>
  {:else}
    <ul class="devices">
      {#each devices as device (device.deviceId)}
        {let bondedName = bondedBluetoothName(device)}
        <li>
          <div class="summary">
            <div class="type-row">
              <span
                aria-hidden="true"
                class={device.type === 'tcp' ? 'i-mdi-lan-connect' : 'i-mdi-bluetooth'}
              ></span>
              <h2>
                {device.type === 'tcp' ? m.tcp_device_type() : m.bluetooth_spp_device_type()}
              </h2>
              <div class="connection-status">
                {#if device.enabled}
                  <span class="sr-only">{m.device_connection_status_unknown()}</span>
                  <StatusPill label="—" />
                {:else}
                  <StatusPill label={m.device_disabled()} />
                {/if}
              </div>
            </div>
            {#if device.type === 'tcp'}
              <p class="endpoint">{device.host}:{device.port}</p>
            {:else}
              <p class="endpoint bluetooth">{bondedName ?? device.address}</p>
              {#if bondedName}
                <p class="address">{device.address}</p>
              {/if}
              {#if device.serviceUuid}
                <p class="service-uuid">
                  <span>{m.custom_service_uuid()}</span>
                  <code>{device.serviceUuid}</code>
                </p>
              {/if}
            {/if}
          </div>
          <label class={['enabled-row', { pending: pendingDeviceIds.includes(device.deviceId) }]}>
            <span>{m.device_enabled()}</span>
            <span class="checkbox-control">
              <input
                type="checkbox"
                role="switch"
                checked={device.enabled}
                disabled={pendingDeviceIds.includes(device.deviceId)}
                onchange={(event) => void requestEnabledChange(event, device)}
              />
              <span aria-hidden="true" class="checkbox-visual">
                <span class="i-mdi-check-bold"></span>
              </span>
            </span>
          </label>
          {#if failedDeviceIds.includes(device.deviceId)}
            <p class="error" role="alert">{m.update_device_error()}</p>
          {/if}
          <a
            class="edit-link"
            aria-label={m.edit_external_device({ endpoint: deviceEndpoint(device) })}
            href={resolve('/settings/devices/[deviceId]', {
              deviceId: String(device.deviceId),
            })}
          >
            <span>{m.edit_connection()}</span>
            <span aria-hidden="true" class="i-mdi-chevron-right"></span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</ScreenScaffold>

<style>
  .devices,
  .skeletons {
    display: grid;
    width: min(100%, 26rem);
    margin: 0 auto;
    padding: 0;
    gap: var(--space-3);
  }

  h2,
  p {
    margin: 0;
  }

  .devices {
    list-style: none;
  }

  .devices li {
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
  }

  .summary {
    padding: 0.875rem var(--space-5) var(--space-3);
  }

  .type-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-block-end: 0.125rem;
    color: var(--color-text-muted);
  }

  .type-row > :first-child {
    flex: 0 0 auto;
    font-size: 1.375rem;
    line-height: 1;
  }

  .type-row h2 {
    font: var(--text-section-title);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .connection-status {
    --status-pill-font-size: 0.9375rem;

    margin-inline-start: auto;
  }

  .endpoint {
    color: var(--color-text);
    font: 600 1.375rem / 1.25 var(--font-numeric);
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }

  .endpoint.bluetooth {
    font-family: var(--font-ui);
    font-variant-numeric: normal;
  }

  .address,
  code {
    overflow-wrap: anywhere;
  }

  .address {
    color: var(--color-text-muted);
    font: 500 1.0625rem / 1.3 var(--font-numeric);
  }

  .service-uuid {
    display: grid;
    margin-block-start: var(--space-2);
    color: var(--color-text-muted);
    font: var(--text-caption);
    gap: var(--space-1);
  }

  .enabled-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    min-height: var(--target-min);
    padding: var(--space-2) var(--space-5);
    border-block-start: 1px solid var(--color-separator);
    color: var(--color-text);
    font: var(--text-row-label);
    cursor: pointer;
  }

  .checkbox-control {
    position: relative;
    display: inline-flex;
    flex: 0 0 auto;
    width: 1.5rem;
    height: 1.5rem;
  }

  .checkbox-control input {
    position: absolute;
    z-index: 1;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .checkbox-visual {
    display: inline-flex;
    width: 100%;
    height: 100%;
    align-items: center;
    justify-content: center;
    border: 2px solid var(--color-border-strong);
    border-radius: 0.1875rem;
    background: var(--color-card-surface);
    color: var(--color-white);
    pointer-events: none;
  }

  .checkbox-visual > span {
    font-size: 1.25rem;
    line-height: 1;
    opacity: 0;
  }

  .checkbox-control input:checked + .checkbox-visual {
    border-color: var(--color-action-primary-surface);
    background: var(--color-action-primary-surface);
  }

  .checkbox-control input:checked + .checkbox-visual > span {
    opacity: 1;
  }

  .checkbox-control input:focus-visible + .checkbox-visual {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  .checkbox-control input:disabled {
    cursor: wait;
  }

  .checkbox-control input:disabled + .checkbox-visual {
    opacity: 0.55;
  }

  .enabled-row.pending {
    cursor: wait;
  }

  .error {
    padding: 0 var(--space-5) var(--space-2);
    color: var(--color-danger-subtle-text);
    font: var(--text-caption);
  }

  .edit-link {
    display: flex;
    align-items: center;
    min-height: var(--target-flight);
    padding: var(--space-2) var(--space-4) var(--space-2) var(--space-5);
    border-block-start: 1px solid var(--color-separator);
    color: var(--color-text);
    font: var(--text-row-detail);
    text-decoration: none;
  }

  .edit-link :global(.i-mdi-chevron-right) {
    margin-inline-start: auto;
    color: var(--color-text-muted);
    font-size: 1.5rem;
    line-height: 1;
  }

  .edit-link:active {
    background: var(--color-control-surface-pressed);
  }

  .edit-link:focus-visible,
  .add-action:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -3px;
  }

  .add-action {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    width: 100%;
    height: var(--button-height-flight);
    padding-inline: var(--space-4);
    border-radius: var(--radius-control);
    background: var(--color-action-primary-surface);
    color: var(--color-action-primary-text);
    font: var(--text-button-large);
    text-decoration: none;
  }

  .add-action > :first-child {
    font-size: 1.5rem;
    line-height: 1;
  }

  .add-action:active:not(.disabled) {
    background: light-dark(var(--color-blue-600), var(--color-blue-300));
  }

  .add-action.disabled {
    opacity: 0.45;
  }

  .loading-state {
    color: var(--color-text-muted);
  }

  .loading-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: min(100%, 26rem);
    margin: 0 auto var(--space-4);
    padding-inline: var(--space-1);
    font: var(--text-row-detail);
  }

  .loading-icon {
    flex: 0 0 auto;
    font-size: 1.5rem;
    animation: devices-loading-spin 900ms linear infinite;
  }

  .skeleton-card {
    padding: var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    animation: devices-loading-pulse 1.4s ease-in-out infinite;
  }

  .skeleton-card span {
    display: block;
    width: 40%;
    height: var(--space-3);
    margin-block-end: var(--space-2);
    border-radius: var(--space-1);
    background: var(--color-control-surface-pressed);
  }

  .skeleton-card .skeleton-endpoint {
    width: 70%;
    height: var(--space-5);
  }

  .empty-state {
    display: flex;
    min-height: 20rem;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-5);
    color: var(--color-text);
    text-align: center;
  }

  .empty-icon {
    color: var(--color-text-muted);
    font-size: 3rem;
    line-height: 1;
  }

  .empty-state h2 {
    font: 700 1.375rem / 1.25 var(--font-ui);
  }

  .empty-state p {
    max-width: 18rem;
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  @keyframes devices-loading-spin {
    to {
      transform: rotate(1turn);
    }
  }

  @keyframes devices-loading-pulse {
    50% {
      opacity: 0.45;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .loading-icon,
    .skeleton-card {
      animation: none;
    }
  }
</style>

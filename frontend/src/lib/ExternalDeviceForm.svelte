<script lang="ts">
  import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
  import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

  import { m } from '$lib/paraglide/messages.js';

  type TcpExternalDevice = Extract<PublishedExternalDevice, { type: 'tcp' }>;

  type ExternalDeviceFormProps = {
    device?: TcpExternalDevice;
    onSave: (spec: ConnectionSpec) => Promise<void>;
    onDelete?: () => Promise<void>;
  };

  let { device, onSave, onDelete }: ExternalDeviceFormProps = $props();
  let host = $derived(device?.host ?? '');
  let port = $derived(device ? String(device.port) : '');
  let submitted = $state(false);
  let pending = $state(false);
  let commandFailed = $state(false);
  let confirmingDelete = $state(false);
  let deletePending = $state(false);
  let deleteFailed = $state(false);
  const trimmedHost = $derived(host.trim());
  const numericPort = $derived(Number(port));
  const validPort = $derived(/^\d+$/.test(port) && numericPort >= 1 && numericPort <= 65535);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    submitted = true;
    if (pending || !trimmedHost || !validPort) return;

    commandFailed = false;
    pending = true;
    try {
      await onSave({ type: 'tcp', host: trimmedHost, port: numericPort });
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
</script>

<form onsubmit={(event) => void submit(event)}>
  <label>
    <span>{m.connection_type()}</span>
    <select disabled>
      <option>{m.tcp_device_type()}</option>
    </select>
  </label>
  <label>
    <span>{m.tcp_host()}</span>
    <input
      name="host"
      aria-invalid={submitted && !trimmedHost}
      aria-describedby={submitted && !trimmedHost ? 'host-error' : undefined}
      bind:value={host}
    />
    {#if submitted && !trimmedHost}
      <span id="host-error" class="error" role="alert">{m.tcp_host_error()}</span>
    {/if}
  </label>
  <label>
    <span>{m.tcp_port()}</span>
    <input
      name="port"
      inputmode="numeric"
      aria-invalid={submitted && !validPort}
      aria-describedby={submitted && !validPort ? 'port-error' : undefined}
      bind:value={port}
    />
    {#if submitted && !validPort}
      <span id="port-error" class="error" role="alert">{m.tcp_port_error()}</span>
    {/if}
  </label>
  <button type="submit" disabled={pending}
    >{device ? m.save_external_device() : m.add_external_device()}</button
  >
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
      {m.confirm_delete_external_device({ endpoint: `${device.host}:${device.port}` })}
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
  form,
  label {
    display: grid;
  }

  form {
    max-width: 30rem;
    gap: 1rem;
  }

  label {
    gap: 0.25rem;
  }

  .error {
    margin: 0;
    color: light-dark(var(--color-red-700), var(--color-red-300));
  }

  input,
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

<script lang="ts">
  import type { ImportAirspaceResult } from '$lib/client';
  import type { AirspaceLoadError } from '$lib/protocol/generated/AirspaceLoadError';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';

  import { m } from '$lib/paraglide/messages.js';

  type Props = {
    status: AirspaceStatus;
    onImport: () => Promise<ImportAirspaceResult>;
    onRemove: () => Promise<void>;
  };

  type MutationState = { type: 'idle' } | { type: 'pending' } | { type: 'failed'; message: string };
  type AirspaceCommandErrorKind =
    | 'pickerFailed'
    | 'readFailed'
    | 'parseFailed'
    | 'geometryFailed'
    | 'storageFailed'
    | 'driverStopped'
    | 'busy';

  let { status, onImport, onRemove }: Props = $props();
  let mutation = $state.raw<MutationState>({ type: 'idle' });
  const pending = $derived(mutation.type === 'pending');

  async function mutate(action: () => Promise<unknown>): Promise<void> {
    mutation = { type: 'pending' };
    try {
      await action();
      mutation = { type: 'idle' };
    } catch (error) {
      mutation = { type: 'failed', message: commandErrorMessage(error) };
    }
  }

  function commandErrorKind(error: unknown): AirspaceCommandErrorKind | null {
    if (typeof error !== 'object' || error === null || !('kind' in error)) return null;

    switch (error.kind) {
      case 'pickerFailed':
      case 'readFailed':
      case 'parseFailed':
      case 'geometryFailed':
      case 'storageFailed':
      case 'driverStopped':
      case 'busy':
        return error.kind;
      default:
        return null;
    }
  }

  function commandErrorMessage(error: unknown): string {
    switch (commandErrorKind(error)) {
      case 'pickerFailed':
        return m.airspace_command_picker_failed();
      case 'readFailed':
        return m.airspace_command_read_failed();
      case 'parseFailed':
        return m.airspace_command_parse_failed();
      case 'geometryFailed':
        return m.airspace_command_geometry_failed();
      case 'storageFailed':
        return m.airspace_command_storage_failed();
      case 'driverStopped':
        return m.airspace_command_driver_stopped();
      case 'busy':
        return m.airspace_command_busy();
      default:
        return m.airspace_command_unknown();
    }
  }

  function loadErrorMessage(error: AirspaceLoadError): string {
    switch (error) {
      case 'readFailed':
        return m.airspace_unavailable_read();
      case 'parseFailed':
        return m.airspace_unavailable_parse();
      case 'geometryFailed':
        return m.airspace_unavailable_geometry();
    }
  }
</script>

<fieldset>
  <legend class:sr-only={status.type === 'none'}>{m.airspace_label()}</legend>
  {#if status.type === 'none'}
    <div class="empty-state">
      <span aria-hidden="true" class="i-mdi-vector-square"></span>
      <p class="empty-title">{m.airspace_none()}</p>
      <p class="empty-description">{m.airspace_none_description()}</p>
    </div>
    <button type="button" disabled={pending} onclick={() => void mutate(onImport)}>
      {m.airspace_import()}
    </button>
  {:else}
    <p>{status.sourceName ?? m.airspace_source_fallback()}</p>
    {#if status.type === 'active'}
      <p>
        {status.airspaceCount === 1
          ? m.airspace_count_one()
          : m.airspace_count({ count: status.airspaceCount })}
      </p>
    {:else}
      <p>{loadErrorMessage(status.error)}</p>
    {/if}
    <div class="actions">
      <button type="button" disabled={pending} onclick={() => void mutate(onImport)}>
        {m.airspace_replace()}
      </button>
      <button type="button" disabled={pending} onclick={() => void mutate(onRemove)}>
        {m.airspace_remove()}
      </button>
    </div>
  {/if}
  {#if mutation.type === 'failed'}
    <p role="alert">{mutation.message}</p>
  {/if}
</fieldset>

<style>
  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-block-end: 0.75rem;
    font-weight: 600;
  }

  p {
    margin-block: 0 0.75rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-5);
    text-align: center;
  }

  .empty-state > span {
    color: var(--color-text-muted);
    font-size: 3rem;
  }

  .empty-title,
  .empty-description {
    margin: 0;
  }

  .empty-title {
    color: var(--color-text);
    font: 700 1.375rem / 1.3 var(--font-ui);
  }

  .empty-description {
    max-width: 18rem;
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  button {
    min-height: 3rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }
</style>

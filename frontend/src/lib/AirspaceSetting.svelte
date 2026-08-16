<script lang="ts">
  import type { ImportAirspaceResult } from '$lib/client';
  import type { AirspaceLoadError } from '$lib/protocol/generated/AirspaceLoadError';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';

  import { m } from '$lib/paraglide/messages.js';
  import Button from './Button.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import StatusPill from './StatusPill.svelte';

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
  let removeDialogOpen = $state(false);
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

  function confirmRemoval(): void {
    void mutate(onRemove);
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

{#snippet actions()}
  <Button disabled={pending} size="large" style="width: 100%" onclick={() => void mutate(onImport)}>
    <span aria-hidden="true" class="i-mdi-file-import-outline action-icon replace-icon"></span>
    {status.type === 'none' ? m.airspace_import() : m.airspace_replace()}
  </Button>
{/snippet}

<ScreenScaffold
  {actions}
  backHref="/settings"
  backLabel={m.back_to_settings()}
  title={m.airspace_label()}
>
  <fieldset>
    <legend class="sr-only">{m.airspace_label()}</legend>
    {#if status.type === 'none'}
      <div class="empty-state">
        <span aria-hidden="true" class="i-mdi-vector-square"></span>
        <p class="empty-title">{m.airspace_none()}</p>
        <p class="empty-description">{m.airspace_none_description()}</p>
      </div>
    {:else}
      <section class="source-summary" aria-labelledby="current-source-heading">
        <h2 id="current-source-heading">{m.airspace_current_source()}</h2>
        <dl>
          <div class="source-row">
            <dt>{m.airspace_file_label()}</dt>
            <dd>{status.sourceName ?? m.airspace_source_fallback()}</dd>
          </div>
          {#if status.type === 'active'}
            <div class="source-row">
              <dt>{m.airspaces_heading()}</dt>
              <dd class="numeric">{status.airspaceCount}</dd>
            </div>
          {/if}
          <div class="source-row">
            <dt>{m.state_label()}</dt>
            <dd>
              {#if status.type === 'active'}
                <StatusPill label={m.airspace_active()} tone="success" />
              {:else}
                <StatusPill
                  icon="i-mdi-alert-circle"
                  label={m.unavailable_value()}
                  tone="danger-subtle"
                />
              {/if}
            </dd>
          </div>
        </dl>
        {#if status.type === 'unavailable'}
          <p class="source-error">
            <span aria-hidden="true" class="i-mdi-alert-circle-outline"></span>
            <span>{loadErrorMessage(status.error)}</span>
          </p>
        {/if}
      </section>
      <p class="source-help">{m.airspace_replace_description()}</p>
      <section class="remove-source" aria-labelledby="remove-source-heading">
        <h2 id="remove-source-heading">{m.airspace_remove_heading()}</h2>
        <p>{m.airspace_remove_description()}</p>
        <Button
          disabled={pending}
          style="width: 100%"
          variant="destructive-outline"
          onclick={() => (removeDialogOpen = true)}
        >
          <span aria-hidden="true" class="i-mdi-delete-outline action-icon"></span>
          {m.airspace_remove()}
        </Button>
      </section>
    {/if}
    {#if mutation.type === 'failed'}
      <p role="alert">{mutation.message}</p>
    {/if}
  </fieldset>
</ScreenScaffold>

{#if status.type !== 'none'}
  <ConfirmDialog
    bind:open={removeDialogOpen}
    title={m.airspace_remove_confirm_title({
      sourceName: status.sourceName ?? m.airspace_source_fallback(),
    })}
    description={m.airspace_remove_confirm_description()}
    cancelLabel={m.cancel()}
    confirmLabel={m.airspace_remove_confirm()}
    onCancel={() => (removeDialogOpen = false)}
    onConfirm={confirmRemoval}
  />
{/if}

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

  .source-summary {
    margin-block-end: var(--space-3);
  }

  h2 {
    margin: 0 var(--space-1) var(--space-2);
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  dl {
    margin: 0;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
  }

  .source-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    min-height: var(--target-min);
    padding: var(--space-2) var(--space-5);
  }

  .source-row + .source-row {
    border-block-start: 1px solid var(--color-separator);
  }

  dt {
    font: var(--text-row-label);
  }

  dd {
    margin: 0;
    font: var(--text-row-detail);
    font-weight: 600;
    text-align: end;
  }

  dd.numeric {
    font: var(--text-row-value);
    font-variant-numeric: tabular-nums;
  }

  .source-error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-3) 0 0;
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-control);
    background: var(--color-danger-subtle-surface);
    color: var(--color-danger-subtle-text);
    font: var(--text-body);
    font-weight: 500;
  }

  .source-error > :first-child {
    flex: 0 0 auto;
    font-size: 1.5rem;
    line-height: 1;
  }

  .source-help,
  .remove-source p {
    color: var(--color-text-muted);
    font: 400 1rem / 1.5 var(--font-ui);
  }

  .source-help {
    margin: 0 var(--space-1) var(--space-6);
  }

  .remove-source {
    padding-block-start: var(--space-6);
    border-block-start: 1px solid var(--color-separator);
  }

  .remove-source h2 {
    color: var(--color-danger-subtle-text);
  }

  .remove-source p {
    margin: 0 var(--space-1) var(--space-3);
  }

  .action-icon {
    font-size: 1.5rem;
    line-height: 1;
  }

  .replace-icon {
    font-size: 1.75rem;
  }
</style>

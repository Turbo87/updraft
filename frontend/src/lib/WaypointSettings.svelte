<script lang="ts">
  import type { ImportWaypointsResult } from '$lib/client';
  import type { WaypointStatus } from '$lib/protocol/generated/WaypointStatus';

  import Button from './Button.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    status: WaypointStatus;
    onImport: () => Promise<ImportWaypointsResult>;
    onRemove: (name: string) => Promise<void>;
  };

  let { status, onImport, onRemove }: Props = $props();
  let pending = $state(false);
  let error = $state('');
  let removeName = $state('');
  let removeOpen = $state(false);

  async function importFile() {
    pending = true;
    error = '';
    try {
      await onImport();
    } catch (cause) {
      error = cause === 'parseFailed' ? m.waypoints_parse_failed() : m.waypoints_command_failed();
    } finally {
      pending = false;
    }
  }
  async function removeFile() {
    pending = true;
    error = '';
    try {
      await onRemove(removeName);
    } catch {
      error = m.waypoints_command_failed();
    } finally {
      removeOpen = false;
      pending = false;
    }
  }
</script>

{#snippet actions()}
  <Button disabled={pending} size="large" style="width: 100%" onclick={importFile}>
    <span aria-hidden="true" class="i-mdi-file-import-outline"></span>
    {m.waypoints_import()}
  </Button>
{/snippet}

<ScreenScaffold
  {actions}
  backHref="/settings"
  backLabel={m.back_to_settings()}
  title={m.waypoints_heading()}
>
  <p class="help">{m.waypoints_import_help()}</p>
  {#if status.sources.length === 0}
    <p>{m.waypoints_none()}</p>
  {/if}
  {#each status.sources as source (source.sourceName)}
    <section>
      <h2>{source.sourceName}</h2>
      {#if source.type === 'active'}
        <p>
          {m.waypoints_import_count({ count: source.waypointCount })}
        </p>
        {#if source.warnings.length}
          <details>
            <summary>{m.waypoints_warnings({ count: source.warnings.length })}</summary>
            <ul>
              {#each source.warnings as warning (warning)}
                <li>
                  {warning.line === null
                    ? warning.message
                    : m.waypoints_warning_line({ line: warning.line, message: warning.message })}
                </li>
              {/each}
            </ul>
          </details>
        {/if}
      {:else}
        <p>
          {source.error === 'readFailed' ? m.waypoints_read_failed() : m.waypoints_parse_failed()}
        </p>
      {/if}
      <Button
        disabled={pending}
        variant="destructive-outline"
        onclick={() => {
          removeName = source.sourceName;
          removeOpen = true;
        }}>{m.waypoints_remove_file()}</Button
      >
    </section>
  {/each}
  {#if error}<p role="alert">{error}</p>{/if}
</ScreenScaffold>

<ConfirmDialog
  bind:open={removeOpen}
  title={m.waypoints_remove_title({ name: removeName })}
  description={m.waypoints_remove_description()}
  cancelLabel={m.cancel()}
  confirmLabel={m.waypoints_remove_confirm()}
  {pending}
  onCancel={() => {
    removeOpen = false;
  }}
  onConfirm={removeFile}
/>

<style>
  .help {
    color: var(--color-text-muted);
  }
  section {
    padding-block: var(--space-4);
    border-block-end: 1px solid var(--color-border);
  }
  h2 {
    margin: 0;
    font: var(--text-row-label);
    overflow-wrap: anywhere;
  }
  p {
    margin-block: var(--space-3);
  }
  summary {
    cursor: pointer;
  }
  li {
    overflow-wrap: anywhere;
    margin-block: var(--space-2);
  }
  [role='alert'] {
    color: var(--color-danger-subtle-text);
  }
</style>

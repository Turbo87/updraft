<script lang="ts">
  import type { AirspaceStatus } from './protocol/generated/AirspaceStatus';
  import type { WaypointStatus } from './protocol/generated/WaypointStatus';

  import { Dialog } from 'bits-ui';

  import Button from './Button.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import { m } from './paraglide/messages.js';
  import { getLocale } from './paraglide/runtime.js';
  import ResponsiveCard from './ResponsiveCard.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import StatusPill from './StatusPill.svelte';

  type DatasetType = 'airspace' | 'waypoints';
  type Props = {
    airspace: AirspaceStatus;
    waypoints: WaypointStatus;
    detailsOpen?: boolean;
    onRemove: (type: DatasetType, name: string) => Promise<void>;
  };
  type Source = AirspaceStatus['sources'][number] | WaypointStatus['sources'][number];

  let { airspace, waypoints, detailsOpen = $bindable(false), onRemove }: Props = $props();
  let selected = $state<{ type: DatasetType; name: string }>();
  let opener: HTMLButtonElement | undefined;
  let removeOpen = $state(false);
  let pending = $state(false);
  let error = $state('');
  const groups = $derived(
    [
      {
        type: 'airspace' as const,
        label: m.airspace_label(),
        icon: 'i-mdi-vector-square',
        sources: airspace.sources,
      },
      {
        type: 'waypoints' as const,
        label: m.waypoints_heading(),
        icon: 'i-mdi-map-marker',
        sources: waypoints.sources,
      },
    ].filter((group) => group.sources.length > 0),
  );

  const selectedGroup = $derived(groups.find((group) => group.type === selected?.type));
  const selectedSource = $derived(
    selectedGroup?.sources.find((source) => source.sourceName === selected?.name),
  );

  $effect(() => {
    if (!selectedSource) detailsOpen = false;
  });

  function restoreFocus(event: Event) {
    event.preventDefault();
    if (!removeOpen) opener?.focus();
  }

  async function removeFile() {
    if (!selected || pending) return;
    pending = true;
    error = '';
    try {
      await onRemove(selected.type, selected.name);
      removeOpen = false;
    } catch {
      error = m.data_remove_failed();
    } finally {
      pending = false;
    }
  }

  function compareSources(a: Source, b: Source): number {
    return a.sourceName.localeCompare(b.sourceName, getLocale());
  }

  function metadata(source: Source): string {
    if (source.type === 'disabled') return m.data_imported();
    if (source.type === 'unavailable') {
      switch (source.error) {
        case 'readFailed':
          return m.data_read_failed();
        case 'parseFailed':
          return m.data_parse_failed();
        case 'geometryFailed':
          return m.data_geometry_failed();
      }
    }
    let count: string;
    if ('airspaceCount' in source) {
      count =
        source.airspaceCount === 1
          ? m.data_airspace_one()
          : m.data_airspaces({ count: source.airspaceCount });
    } else {
      count =
        source.waypointCount === 1
          ? m.data_waypoint_one()
          : m.data_waypoints({ count: source.waypointCount });
      if (source.warnings.length) {
        count += ` · ${source.warnings.length === 1 ? m.data_warning_one() : m.data_warnings({ count: source.warnings.length })}`;
      }
    }
    return `${m.data_imported()} · ${count}`;
  }
</script>

<ScreenScaffold backHref="/settings" backLabel={m.back_to_settings()} title={m.data_heading()}>
  {#if groups.length === 0}
    <div class="empty">
      <span aria-hidden="true" class="i-mdi-database-outline"></span>
      <p class="empty-title">{m.data_empty()}</p>
      <p>{m.data_empty_hint()}</p>
    </div>
  {/if}
  {#each groups as group (group.type)}
    <section aria-labelledby={`data-${group.type}`}>
      <h2 id={`data-${group.type}`}>{group.label}</h2>
      <ResponsiveCard>
        <ul class="files">
          {#each group.sources.toSorted(compareSources) as source (source.sourceName)}
            <li
              class:disabled={source.type === 'disabled'}
              class:unavailable={source.type === 'unavailable'}
            >
              <button
                class="file-row"
                onclick={(event) => {
                  selected = { type: group.type, name: source.sourceName };
                  opener = event.currentTarget;
                  detailsOpen = true;
                }}
              >
                <span aria-hidden="true" class={[group.icon, 'type-icon']}></span>
                <span class="description">
                  <span class="name">
                    <span class="filename">{source.sourceName}</span>
                    {#if source.type === 'disabled'}<StatusPill label={m.data_disabled()} />{/if}
                  </span>
                  <span class="metadata">{metadata(source)}</span>
                </span>
                <span aria-hidden="true" class="i-mdi-chevron-right type-icon"></span>
              </button>
            </li>
          {/each}
        </ul>
      </ResponsiveCard>
    </section>
  {/each}
</ScreenScaffold>

<Dialog.Root bind:open={detailsOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="data-dialog-overlay" />
    <Dialog.Content class="data-dialog-content" onCloseAutoFocus={restoreFocus}>
      {#if selectedSource && selectedGroup}
        <header>
          <span aria-hidden="true" class={[selectedGroup.icon, 'type-icon']}></span>
          <div class="description">
            <Dialog.Title class="data-dialog-title" level={2}
              >{selectedSource.sourceName}</Dialog.Title
            >
            <Dialog.Description class="data-dialog-description"
              >{selectedGroup.label}</Dialog.Description
            >
          </div>
          <Dialog.Close class="data-dialog-close" aria-label={m.data_close()}>×</Dialog.Close>
        </header>
        <dl>
          <div>
            <dt>{m.data_source()}</dt>
            <dd>{m.data_imported()}</dd>
          </div>
          <div>
            <dt>{m.data_status()}</dt>
            <dd>{selectedSource.type === 'disabled' ? m.data_disabled() : m.data_enabled()}</dd>
          </div>
          {#if selectedSource.type === 'active'}
            <div>
              <dt>{selectedGroup.label}</dt>
              <dd>
                {'airspaceCount' in selectedSource
                  ? selectedSource.airspaceCount
                  : selectedSource.waypointCount}
              </dd>
            </div>
          {/if}
        </dl>
        {#if selectedSource.type === 'unavailable'}
          <p class="error">{metadata(selectedSource)}</p>
        {:else if selectedSource.type === 'active' && 'warnings' in selectedSource && selectedSource.warnings.length}
          <ul class="diagnostics">
            {#each selectedSource.warnings as warning (warning)}
              <li>
                {warning.line === null
                  ? warning.message
                  : m.waypoints_warning_line({ line: warning.line, message: warning.message })}
              </li>
            {/each}
          </ul>
        {/if}
        <Button
          variant="destructive-outline"
          size="large"
          style="width: 100%"
          onclick={() => {
            detailsOpen = false;
            error = '';
            removeOpen = true;
          }}>{m.data_remove()}</Button
        >
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<ConfirmDialog
  bind:open={removeOpen}
  title={m.waypoints_remove_title({ name: selected?.name ?? '' })}
  description={m.data_remove_description()}
  cancelLabel={m.cancel()}
  confirmLabel={m.waypoints_remove_confirm()}
  {pending}
  {error}
  onCancel={() => {
    removeOpen = false;
    opener?.focus();
  }}
  onConfirm={removeFile}
/>

<style>
  section + section {
    margin-block-start: 15px;
  }
  h2 {
    margin: 0 var(--space-1) 15px;
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .file-row {
    width: 100%;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: start;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--space-5);
    min-height: var(--target-min);
    padding-block: var(--space-4);
    padding-inline: calc(var(--space-6) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
  }
  .files > li + li {
    border-block-start: 1px solid var(--color-separator);
  }
  .type-icon {
    flex: 0 0 auto;
    font-size: 24px;
    color: var(--color-text-muted);
  }
  .description {
    min-width: 0;
    flex: 1;
  }
  .name {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    --status-pill-font-size: 13px;
  }
  .filename {
    overflow-wrap: anywhere;
    font: var(--text-row-label);
  }
  .metadata {
    display: block;
    margin: var(--space-1) 0 0;
    color: var(--color-text-muted);
    font: 400 15px/1.4 var(--font-ui);
  }
  .disabled .filename {
    color: var(--color-text-muted);
  }
  .disabled .type-icon {
    color: var(--color-text-faint);
  }
  .unavailable .type-icon,
  .unavailable .metadata {
    color: var(--color-error-text);
  }
  .empty {
    padding-block: var(--space-8);
    text-align: center;
    color: var(--color-text-muted);
  }
  .empty > span {
    display: inline-block;
    font-size: 48px;
  }
  .empty-title {
    color: var(--color-text);
    font: var(--text-row-label);
  }
  .file-row:active {
    background: var(--color-control-surface-raised-pressed);
  }
  .file-row:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }
  :global(.data-dialog-overlay) {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: var(--color-scrim);
  }
  :global(.data-dialog-content) {
    position: fixed;
    z-index: 101;
    top: 50%;
    left: 50%;
    translate: -50% -50%;
    box-sizing: border-box;
    width: min(calc(100% - 48px), 544px);
    max-height: calc(100dvh - 48px);
    overflow-y: auto;
    padding: 20px 24px 24px;
    border-radius: 16px;
    background: var(--color-card-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-modal);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  :global(.data-dialog-title) {
    margin: 0;
    overflow-wrap: anywhere;
    font: var(--text-screen-title);
  }
  :global(.data-dialog-description) {
    margin: 0;
    color: var(--color-text-muted);
  }
  :global(.data-dialog-close) {
    flex-shrink: 0;
    width: 48px;
    height: 48px;
    border: 0;
    background: transparent;
    color: var(--color-text-muted);
    font-size: 32px;
    cursor: pointer;
  }
  dl {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-4);
    padding-block-start: var(--space-4);
    border-block-start: 1px solid var(--color-separator);
  }
  dt {
    color: var(--color-text-muted);
  }
  dd {
    margin: 0;
    font: 500 17px/1.35 var(--font-ui);
  }
  .diagnostics {
    list-style: disc;
    padding-inline-start: var(--space-5);
    margin-block: var(--space-4);
    overflow-wrap: anywhere;
  }
  .error {
    color: var(--color-error-text);
  }
  @media (min-width: 545px) {
    dl {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
</style>

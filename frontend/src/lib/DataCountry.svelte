<script lang="ts">
  import type {
    BasemapStatus,
    EnrouteBasemapEntry,
    EnrouteDownloadStatus,
    UpdraftClient,
  } from './client';

  import { onDestroy } from 'svelte';

  import Button from './Button.svelte';
  import IconButton from './IconButton.svelte';
  import { m } from './paraglide/messages.js';
  import { getLocale } from './paraglide/runtime.js';
  import ResponsiveCard from './ResponsiveCard.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    country: string;
    entries: EnrouteBasemapEntry[];
    basemaps: BasemapStatus | null;
    downloads: EnrouteDownloadStatus[] | null;
    stateError?: boolean;
    awaitingLibrary?: boolean;
    client: Pick<
      UpdraftClient,
      'getEnrouteBasemapUpdates' | 'downloadEnrouteBasemaps' | 'cancelEnrouteDownload'
    >;
    onBack: () => void;
    onDownloaded: () => void;
  };
  let {
    country,
    entries,
    basemaps,
    downloads,
    stateError = false,
    awaitingLibrary = false,
    client,
    onBack,
    onDownloaded,
  }: Props = $props();
  const componentId = $props.id();
  let selection = $state<string[]>([]);
  let pending = $state(false);
  let error = $state('');
  let updates = $state.raw<string[] | null>(null);
  let checkError = $state(false);
  let retry = $state(0);
  let disposed = false;
  onDestroy(() => {
    disposed = true;
  });
  let countryName = $derived(new Intl.DisplayNames(getLocale(), { type: 'region' }).of(country)!);
  let ready = $derived(basemaps !== null && downloads !== null && !stateError);
  let rows = $derived(
    entries.map((entry) => {
      let installed = basemaps?.sources.some(
        (source) => source.sourceName === `enroute/${entry.path}`,
      );
      let transfer = downloads?.find((download) => download.path === entry.path);
      let busy = transfer?.type === 'queued' || transfer?.type === 'downloading';
      let update = updates?.includes(entry.path);
      return {
        entry,
        installed,
        transfer,
        busy,
        update,
        selectable: !busy && (!installed || update),
        name:
          entries.length === 1
            ? countryName
            : entry.path
                .split('/')
                .at(-1)!
                .replace(/\.mbtiles$/, ''),
      };
    }),
  );
  let selected = $derived(
    rows.filter((row) => row.selectable && selection.includes(row.entry.path)),
  );
  function size(bytes: number) {
    return new Intl.NumberFormat(getLocale(), {
      style: 'unit',
      unit: 'megabyte',
      maximumFractionDigits: 1,
    }).format(bytes / 1_000_000);
  }
  $effect(() => {
    void country;
    selection = [];
    error = '';
  });
  $effect(() => {
    let allowed = rows.filter((row) => row.selectable).map((row) => row.entry.path);
    if (selection.some((path) => !allowed.includes(path))) {
      selection = selection.filter((path) => allowed.includes(path));
    }
  });
  $effect(() => {
    void retry;
    let inventory = basemaps;
    let catalog = entries;
    let active = true;
    updates = null;
    checkError = false;
    if (
      inventory &&
      catalog.some((entry) =>
        inventory.sources.some((source) => source.sourceName === `enroute/${entry.path}`),
      )
    ) {
      client.getEnrouteBasemapUpdates().then(
        (paths) => {
          if (active) updates = paths;
        },
        () => {
          if (active) checkError = true;
        },
      );
    }
    return () => {
      active = false;
    };
  });
  async function download() {
    let submittedCountry = country;
    pending = true;
    error = '';
    try {
      await client.downloadEnrouteBasemaps(selected.map((row) => row.entry.path));
      if (!disposed && country === submittedCountry) onDownloaded();
    } catch {
      error = m.data_download_start_failed();
    } finally {
      pending = false;
    }
  }
  async function cancel(path: string) {
    error = '';
    try {
      await client.cancelEnrouteDownload(path);
    } catch {
      error = m.data_download_cancel_failed();
    }
  }
</script>

{#snippet downloadAction()}
  <Button
    loading={pending || awaitingLibrary}
    disabled={!ready || selected.length === 0}
    onclick={download}
    style="width: 100%"
  >
    {m.data_download()}{selected.length
      ? ` · ${size(selected.reduce((total, row) => total + row.entry.size, 0))}`
      : ''}
  </Button>
{/snippet}

<ScreenScaffold
  {onBack}
  backLabel={m.back_to_add_data()}
  title={countryName}
  responsiveActions
  actions={downloadAction}
>
  {#if stateError}<p role="alert">{m.data_download_state_failed()}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
  {#if checkError}
    <p role="alert">{m.data_update_check_failed()}</p>
    <Button variant="secondary" onclick={() => retry++}>{m.retry()}</Button>
  {/if}
  {#if entries.length === 0}
    <p role="status">{m.data_catalog_unavailable()}</p>
  {:else}
    <section aria-labelledby={`${componentId}-basemaps`}>
      <h2 id={`${componentId}-basemaps`}>{m.data_basemap()}</h2>
      <ResponsiveCard>
        {#each rows as row (row.entry.path)}
          {#snippet description()}
            <span class="description">
              <strong>{row.name}</strong>
              <span
                >{new Intl.DateTimeFormat(getLocale(), {
                  dateStyle: 'medium',
                  timeZone: 'UTC',
                }).format(new Date(row.entry.publicationDate))} · {size(row.entry.size)}</span
              >
              {#if row.transfer?.type === 'downloading'}
                <span
                  >{m.data_downloading()} · {m.data_download_progress({
                    done: size(row.transfer.downloaded),
                    total: size(row.transfer.total),
                  })}</span
                >
                <progress
                  value={row.transfer.downloaded}
                  max={row.transfer.total}
                  aria-label={row.name}
                ></progress>
              {:else if row.busy}<span>{m.data_queued()}</span>
              {:else if row.update}<span>{m.data_update_available()}</span>
              {:else if row.installed}<span
                  >{updates === null && !checkError
                    ? m.data_checking_updates()
                    : m.data_installed()}</span
                >{/if}
            </span>
          {/snippet}
          {#if row.selectable}
            <label class="dataset">
              {@render description()}
              <span class="control">
                <input
                  type="checkbox"
                  aria-label={row.name}
                  value={row.entry.path}
                  bind:group={selection}
                  disabled={!ready || pending || awaitingLibrary}
                />
                <span class="check i-mdi-check-bold" aria-hidden="true"></span>
              </span>
            </label>
          {:else}
            <div class="dataset">
              {@render description()}
              {#if row.busy}
                <IconButton
                  icon="i-mdi-close"
                  label={m.data_cancel_download({ name: row.name })}
                  onclick={() => cancel(row.entry.path)}
                />
              {/if}
            </div>
          {/if}
        {/each}
      </ResponsiveCard>
    </section>
  {/if}
</ScreenScaffold>

<style>
  h2 {
    margin: var(--space-4) var(--space-1);
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .dataset {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: var(--target-min);
    padding-block: var(--space-4);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
  }
  .dataset + .dataset {
    border-block-start: 1px solid var(--color-separator);
  }
  label {
    cursor: pointer;
  }
  .description {
    display: grid;
    gap: var(--space-2);
    flex: 1;
    min-width: 0;
    color: var(--color-text-muted);
    font: var(--text-row-detail);
    overflow-wrap: anywhere;
  }
  strong {
    color: var(--color-text);
    font: var(--text-row-label);
  }
  .control {
    position: relative;
    display: grid;
    place-items: center;
    flex: 0 0 48px;
    height: 48px;
  }
  input {
    appearance: none;
    margin: 0;
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-text-muted);
    border-radius: 6px;
    cursor: pointer;
  }
  input:checked {
    border-color: var(--color-action-primary-surface);
    background: var(--color-action-primary-surface);
  }
  input:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 4px;
  }
  input:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .check {
    position: absolute;
    pointer-events: none;
    color: var(--color-action-primary-text);
    width: 18px;
    height: 18px;
    visibility: hidden;
  }
  input:checked + .check {
    visibility: visible;
  }
  progress {
    width: 100%;
    height: 6px;
    appearance: none;
    border: 0;
    border-radius: 3px;
    overflow: hidden;
    background: var(--color-separator);
  }
  progress::-webkit-progress-bar {
    background: var(--color-separator);
  }
  progress::-webkit-progress-value {
    background: var(--color-action-primary-surface);
  }
  progress::-moz-progress-bar {
    background: var(--color-action-primary-surface);
  }
  p {
    color: var(--color-text-muted);
    font: var(--text-body);
  }
</style>

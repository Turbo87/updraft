<script lang="ts">
  import type {
    BasemapFileDetails,
    BasemapStatus,
    EnrouteCatalogStatus,
    EnrouteDownloadStatus,
    TerrainStatus,
    UpdraftClient,
  } from './client';
  import type { AirspaceStatus } from './protocol/generated/AirspaceStatus';
  import type { WaypointStatus } from './protocol/generated/WaypointStatus';
  import type { DataActivation } from './stores/data-activation.svelte';

  import { onDestroy, tick, untrack } from 'svelte';
  import { MediaQuery } from 'svelte/reactivity';
  import { Dialog } from 'bits-ui';

  import Button from './Button.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import DataCatalog from './DataCatalog.svelte';
  import DataCountry from './DataCountry.svelte';
  import DataUpdateCheckFailure from './DataUpdateCheckFailure.svelte';
  import IconButton from './IconButton.svelte';
  import { m } from './paraglide/messages.js';
  import { getLocale } from './paraglide/runtime.js';
  import ResponsiveCard from './ResponsiveCard.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import StatusPill from './StatusPill.svelte';
  import { DataImport } from './stores/data-import.svelte';

  const componentId = $props.id();

  type DatasetType = 'airspace' | 'waypoints' | 'basemap' | 'terrain';
  type Props = {
    catalog: EnrouteCatalogStatus | null;
    catalogError?: boolean;
    updateCheckError?: boolean;
    updates?: string[] | null;
    downloads?: EnrouteDownloadStatus[] | null;
    downloadError?: boolean;
    onDownload: (paths: string[]) => Promise<void>;
    onCancelDownload: (path: string) => Promise<void>;
    onRetryCatalog: () => Promise<void>;
    onCheckBasemapUpdates: () => Promise<string[]>;
    onReadBasemapDetails: (name: string) => Promise<BasemapFileDetails>;
    importer: Pick<UpdraftClient, 'selectDataFile' | 'importDataFile' | 'discardDataFile'>;
    airspace: AirspaceStatus;
    waypoints: WaypointStatus;
    basemaps?: BasemapStatus | null;
    basemapError?: boolean;
    terrain?: TerrainStatus | null;
    terrainError?: boolean;
    onRemove: (type: DatasetType, name: string) => Promise<void>;
    activation: Pick<DataActivation, 'isEnabled' | 'hasError' | 'setEnabled' | 'pending'>;
  };
  type Source =
    | AirspaceStatus['sources'][number]
    | WaypointStatus['sources'][number]
    | BasemapStatus['sources'][number]
    | TerrainStatus['sources'][number];

  let {
    catalog,
    catalogError = false,
    updateCheckError = false,
    updates = null,
    downloads = [],
    downloadError = false,
    onDownload,
    onCancelDownload,
    onRetryCatalog,
    onCheckBasemapUpdates,
    onReadBasemapDetails,
    importer,
    airspace,
    waypoints,
    basemaps = { generation: 0, sources: [] },
    basemapError = false,
    terrain = { generation: 0, sources: [] },
    terrainError = false,
    onRemove,
    activation,
  }: Props = $props();
  let catalogOpen = $state(false);
  let updatesOpen = $state(false);
  let updatePending = $state(false);
  let libraryScroll = 0;
  let availableUpdates = $derived(
    catalog?.cached?.entries.filter(
      (entry) =>
        updates?.includes(entry.path) &&
        basemaps?.sources.some((source) => source.sourceName === `enroute/${entry.path}`),
    ) ?? [],
  );
  let idleUpdates = $derived(
    availableUpdates.filter(
      (entry) =>
        !downloads?.some((download) => download.path === entry.path && download.type !== 'failed'),
    ),
  );
  let catalogCountry = $state<string>();
  let acceptedDownloads = $state<string[] | null>(null);
  let requestedDownloads: string[] = [];
  const downloadClient = {
    getEnrouteBasemapUpdates: () => onCheckBasemapUpdates(),
    downloadEnrouteBasemaps: (paths: string[]) => {
      requestedDownloads = paths;
      return onDownload(paths);
    },
    cancelEnrouteDownload: (path: string) => onCancelDownload(path),
  };
  let countryEntries = $derived(
    catalog?.cached?.entries.filter((entry) => entry.countryCode === catalogCountry) ?? [],
  );
  $effect(() => {
    if (
      acceptedDownloads?.every(
        (path) =>
          downloads?.some((download) => download.path === path) ||
          basemaps?.sources.some((source) => source.sourceName === `enroute/${path}`),
      )
    ) {
      acceptedDownloads = null;
      catalogOpen = false;
      catalogCountry = undefined;
      void tick().then(() =>
        libraryContainer.querySelector('main')?.focus({ preventScroll: true }),
      );
    }
  });
  let detailsOpen = $state(false);
  let selected = $state<{ type: DatasetType; name: string }>();
  let opener: HTMLButtonElement | undefined;
  let removeOpen = $state(false);
  let pending = $state(false);
  let error = $state('');
  let downloadActionError = $state('');
  const wideScreen = new MediaQuery('(min-width: 545px)');
  const dataImport = new DataImport(() => ({ importer, airspace, waypoints }));
  onDestroy(() => dataImport.destroy());

  let libraryContainer: HTMLDivElement;
  export function handleBack(): boolean {
    if (dataImport.selection) void dataImport.cancelImport();
    else if (detailsOpen) detailsOpen = false;
    else if (catalogCountry) {
      catalogCountry = undefined;
      acceptedDownloads = null;
    } else if (catalogOpen) {
      catalogOpen = false;
      void tick().then(() => {
        Array.from(libraryContainer.querySelectorAll<HTMLButtonElement>('.add-data'))
          .find((button) => button.checkVisibility())
          ?.focus({ preventScroll: true });
      });
    } else if (updatesOpen) {
      updatesOpen = false;
      void tick().then(() => {
        let main = libraryContainer.querySelector('main')!;
        main.scrollTop = libraryScroll;
        (libraryContainer.querySelector<HTMLButtonElement>('.update-notice') ?? main).focus({
          preventScroll: true,
        });
      });
    } else return false;
    return true;
  }
  async function returnAfterImport() {
    if (!dataImport.scrollTarget) return;
    catalogOpen = false;
    catalogCountry = undefined;
    await tick();
    libraryContainer.querySelector('main')?.focus({ preventScroll: true });
  }
  async function selectImport(opener: HTMLButtonElement) {
    await dataImport.selectFile(opener);
    await returnAfterImport();
  }

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
      {
        type: 'basemap' as const,
        label: m.data_basemap(),
        icon: 'i-mdi-map-outline',
        sources: basemaps?.sources ?? [],
      },
      {
        type: 'terrain' as const,
        label: m.data_terrain(),
        icon: 'i-mdi-terrain',
        sources: terrain?.sources ?? [],
      },
    ].filter(
      (group) => group.sources.length > 0 || (group.type === 'basemap' && downloads?.length),
    ),
  );

  const selectedGroup = $derived(groups.find((group) => group.type === selected?.type));
  let visibleGroups = $derived(
    updatesOpen
      ? groups.filter((group) => group.type === 'basemap' && availableUpdates.length > 0)
      : groups,
  );

  async function startUpdates(paths: string[]) {
    updatePending = true;
    downloadActionError = '';
    try {
      await onDownload(paths);
    } catch {
      downloadActionError = m.data_download_start_failed();
    } finally {
      updatePending = false;
    }
  }
  const selectedSource = $derived<Source | undefined>(
    selectedGroup?.sources.find((source) => source.sourceName === selected?.name),
  );
  const selectedEnabled = $derived(
    selected && selectedSource && activation.isEnabled(selected.type, selectedSource),
  );
  let selectedDownloadEntry = $derived(
    catalog?.cached?.entries.find(
      (entry) =>
        selected?.type === 'basemap' &&
        selected.name === `enroute/${entry.path}` &&
        (selectedSource?.type === 'unavailable' || updates?.includes(entry.path)),
    ),
  );
  let selectedDownload = $derived(
    downloads?.find(
      (download) => selected?.type === 'basemap' && selected.name === `enroute/${download.path}`,
    ),
  );
  let fileDetails = $state.raw<BasemapFileDetails | null>(null);
  let fileDetailsError = $state(false);
  let fileDetailsRetry = $state(0);
  let detailsName = $derived(
    detailsOpen && selected?.type === 'basemap' ? selected.name : undefined,
  );
  let detailsGeneration = $derived(basemaps?.generation);
  let readDetails = $derived(onReadBasemapDetails);
  $effect(() => {
    let name = detailsName;
    let read = readDetails;
    void detailsGeneration;
    void fileDetailsRetry;
    let active = true;
    fileDetails = null;
    fileDetailsError = false;
    if (name)
      untrack(() => read(name)).then(
        (details) => {
          if (active) fileDetails = details;
        },
        () => {
          if (active) fileDetailsError = true;
        },
      );
    return () => {
      active = false;
    };
  });

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

  function displayName(source: Pick<Source, 'sourceName'>, type: DatasetType): string {
    let name = source.sourceName;
    return type === 'basemap' || type === 'terrain' ? name.slice(name.lastIndexOf('/') + 1) : name;
  }

  type FileRow = { sourceName: string; source?: Source; download?: EnrouteDownloadStatus };
  function fileRows(type: DatasetType, sources: Source[]): FileRow[] {
    let transfers = type === 'basemap' ? (downloads ?? []) : [];
    let rows: FileRow[] = sources.map((source) => ({
      sourceName: source.sourceName,
      source,
      download: transfers.find((download) => `enroute/${download.path}` === source.sourceName),
    }));
    for (let download of transfers) {
      let sourceName = `enroute/${download.path}`;
      if (!sources.some((source) => source.sourceName === sourceName))
        rows.push({ sourceName, download });
    }
    if (updatesOpen)
      rows = rows.filter((row) =>
        availableUpdates.some((entry) => row.sourceName === `enroute/${entry.path}`),
      );
    return rows.toSorted((a, b) =>
      displayName(a, type).localeCompare(displayName(b, type), getLocale()),
    );
  }
  function size(bytes: number) {
    return new Intl.NumberFormat(getLocale(), {
      style: 'unit',
      unit: 'megabyte',
      maximumFractionDigits: 1,
    })
      .format(bytes / 1_000_000)
      .replaceAll(' ', '\u00a0');
  }
  function downloadStatus(download: EnrouteDownloadStatus): string {
    if (download.type === 'failed') return m.data_download_failed();
    if (download.type === 'downloading')
      return `${m.data_downloading()} · ${m.data_download_progress({ done: size(download.downloaded), total: size(download.total) })}`;
    let entry = catalog?.cached?.entries.find((entry) => entry.path === download.path);
    if (!entry) return m.data_queued();
    let date = new Intl.DateTimeFormat(getLocale(), {
      dateStyle: 'medium',
      timeZone: 'UTC',
    }).format(new Date(entry.publicationDate));
    return `${m.data_queued()} · ${date} · ${size(entry.size)}`;
  }
  async function downloadAction(download: EnrouteDownloadStatus) {
    downloadActionError = '';
    try {
      if (download.type === 'failed') await onDownload([download.path]);
      else await onCancelDownload(download.path);
    } catch {
      downloadActionError =
        download.type === 'failed'
          ? m.data_download_start_failed()
          : m.data_download_cancel_failed();
    }
  }

  function metadata(
    source: Source,
    type: DatasetType,
    enabled = source.type !== 'disabled',
  ): string {
    if (type === 'basemap' || type === 'terrain')
      return enabled && source.type === 'unavailable' ? m.data_load_failed() : m.data_on_device();
    if (!enabled || source.type === 'disabled') return m.data_imported();
    if (source.type === 'unavailable' && 'error' in source) {
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
    } else if ('waypointCount' in source) {
      count =
        source.waypointCount === 1
          ? m.data_waypoint_one()
          : m.data_waypoints({ count: source.waypointCount });
      if (source.warnings.length) {
        count += ` · ${source.warnings.length === 1 ? m.data_warning_one() : m.data_warnings({ count: source.warnings.length })}`;
      }
    } else return m.data_imported();
    return `${m.data_imported()} · ${count}`;
  }
</script>

{#if catalogOpen && catalogCountry}
  <DataCountry
    country={catalogCountry}
    entries={countryEntries}
    {basemaps}
    {downloads}
    stateError={basemapError || downloadError || catalogError}
    awaitingLibrary={acceptedDownloads !== null}
    client={downloadClient}
    onBack={handleBack}
    onDownloaded={() => (acceptedDownloads = requestedDownloads)}
  />
{:else if catalogOpen}
  <DataCatalog
    status={catalog}
    subscriptionError={catalogError}
    importPending={dataImport.pending}
    importError={dataImport.error}
    onImport={selectImport}
    onRetry={onRetryCatalog}
    onCountry={(country) => (catalogCountry = country)}
    onBack={handleBack}
  />
{/if}
<div hidden={catalogOpen} bind:this={libraryContainer} style="height: 100%">
  <ScreenScaffold
    {...updatesOpen ? { onBack: handleBack } : { backHref: '/settings' as const }}
    backLabel={updatesOpen ? m.back_to_data() : m.back_to_settings()}
    title={updatesOpen ? m.data_updates_heading() : m.data_heading()}
    responsiveActions
  >
    {#snippet actions()}
      {#if updatesOpen}
        <Button
          style="width: 100%"
          size={wideScreen.current ? 'standard' : 'large'}
          loading={updatePending}
          disabled={!idleUpdates.length || downloads === null || downloadError}
          onclick={() => startUpdates(idleUpdates.map((entry) => entry.path))}
          >{m.data_update_all({
            size: size(idleUpdates.reduce((total, entry) => total + entry.size, 0)),
          })}</Button
        >
      {:else}
        <Button
          class="add-data"
          style="width: 100%"
          size={wideScreen.current ? 'standard' : 'large'}
          loading={dataImport.pending}
          disabled={activation.pending}
          onclick={() => {
            catalogCountry = undefined;
            catalogOpen = true;
          }}
        >
          <span aria-hidden="true" class="i-mdi-plus"></span>{m.data_add()}
        </Button>
      {/if}
    {/snippet}
    {#if downloadActionError && !detailsOpen}<p class="error" role="alert">
        {downloadActionError}
      </p>{/if}
    {#if updateCheckError}
      <DataUpdateCheckFailure
        checkedAt={catalog?.cached?.checkedAt}
        refreshing={catalog?.refreshing}
        onRetry={onRetryCatalog}
      />
    {/if}
    {#if !updatesOpen && availableUpdates.length}
      <ResponsiveCard style="margin-block-end: var(--space-4)">
        <button
          class="file-row update-notice"
          onclick={async () => {
            libraryScroll = libraryContainer.querySelector('main')!.scrollTop;
            updatesOpen = true;
            await tick();
            let main = libraryContainer.querySelector('main')!;
            main.scrollTop = 0;
            main.focus({ preventScroll: true });
          }}
        >
          <span aria-hidden="true" class="i-mdi-download type-icon"></span>
          <span class="description"
            >{availableUpdates.length === 1
              ? m.data_update_one()
              : m.data_updates_available({ count: availableUpdates.length })}</span
          >
          <span aria-hidden="true" class="i-mdi-chevron-right type-icon"></span>
        </button>
      </ResponsiveCard>
    {/if}
    {#if updatesOpen && !availableUpdates.length && !updateCheckError && !basemapError}
      <p role="status">{updates === null ? m.data_checking_updates() : m.data_no_updates()}</p>
    {/if}
    {#if downloadError}<p class="error" role="alert">{m.data_download_state_failed()}</p>{/if}
    {#if dataImport.error}<p class="error" role="alert">{dataImport.error}</p>{/if}
    {#if basemapError}<p class="error" role="alert">{m.data_basemap_failed()}</p>
    {:else if basemaps === null}<p role="status">{m.data_basemap_loading()}</p>{/if}
    {#if terrainError}<p class="error" role="alert">{m.data_terrain_failed()}</p>
    {:else if terrain === null}<p role="status">{m.data_terrain_loading()}</p>{/if}
    {#if !updatesOpen && !downloadError && downloads !== null && groups.length === 0 && basemaps !== null && !basemapError && terrain !== null && !terrainError}
      <div class="empty">
        <span aria-hidden="true" class="i-mdi-database-outline"></span>
        <p class="empty-title">{m.data_empty()}</p>
        <p>{m.data_empty_hint()}</p>
      </div>
    {/if}
    {#each visibleGroups as group (group.type)}
      <section aria-labelledby={`${componentId}-${group.type}`}>
        <h2 id={`${componentId}-${group.type}`}>{group.label}</h2>
        <ResponsiveCard>
          <ul class="files">
            {#each fileRows(group.type, group.sources) as row (row.sourceName)}
              {let source = $derived(row.source)}
              {let update = $derived(
                group.type === 'basemap'
                  ? availableUpdates.find((entry) => row.sourceName === `enroute/${entry.path}`)
                  : undefined,
              )}
              {let enabled = $derived(source && activation.isEnabled(group.type, source))}
              <li
                {@attach (element) => {
                  if (
                    !source ||
                    catalogOpen ||
                    source.type === 'disabled' ||
                    dataImport.scrollTarget?.dataType !== group.type ||
                    dataImport.scrollTarget.sourceName !== source.sourceName
                  )
                    return;
                  let status = group.type === 'airspace' ? airspace : waypoints;
                  if (status.generation <= dataImport.scrollTarget.generation) return;
                  element.scrollIntoView({ block: 'nearest', inline: 'nearest' });
                  dataImport.scrollTarget = undefined;
                }}
                class:disabled={!!source && !enabled}
                class:unavailable={source?.type === 'unavailable' && enabled}
                class:download-failed={row.download?.type === 'failed'}
              >
                {#snippet description()}
                  <span aria-hidden="true" class={[group.icon, 'type-icon']}></span>
                  <span class="description">
                    <span class="name">
                      <span class="filename">{displayName(row, group.type)}</span>
                      {#if source && !enabled}<StatusPill label={m.data_disabled()} />{/if}
                    </span>
                    {#if source}<span class="metadata">{metadata(source, group.type, enabled)}</span
                      >{/if}
                    {#if row.download}
                      <span
                        class="download-status"
                        class:active={row.download.type === 'downloading'}
                        >{downloadStatus(row.download)}</span
                      >
                      {#if row.download.type === 'downloading'}
                        <progress
                          value={row.download.downloaded}
                          max={row.download.total}
                          aria-label={displayName(row, group.type)}
                        ></progress>
                      {/if}
                    {:else if update}
                      <span class="download-status"
                        >{m.data_update_available()} · {new Intl.DateTimeFormat(getLocale(), {
                          dateStyle: 'medium',
                          timeZone: 'UTC',
                        }).format(new Date(update.publicationDate))} · {size(update.size)}</span
                      >
                    {/if}
                    {#if source && activation.hasError(group.type, source.sourceName)}<span
                        class="error">{m.data_activation_failed()}</span
                      >{/if}
                  </span>
                {/snippet}
                {#if source}
                  <button
                    class="file-row"
                    disabled={dataImport.pending}
                    onclick={(event) => {
                      selected = { type: group.type, name: row.sourceName };
                      downloadActionError = '';
                      opener = event.currentTarget;
                      detailsOpen = true;
                    }}
                  >
                    {@render description()}
                    {#if !row.download && !update}<span
                        aria-hidden="true"
                        class="i-mdi-chevron-right type-icon"
                      ></span>{/if}
                  </button>
                {:else}
                  <div class="file-row">{@render description()}</div>
                {/if}
                {#if row.download}
                  <span class="download-action">
                    <IconButton
                      icon={row.download.type === 'failed' ? 'i-mdi-refresh' : 'i-mdi-close'}
                      label={row.download.type === 'failed'
                        ? m.data_retry_download({ name: displayName(row, group.type) })
                        : m.data_cancel_download({ name: displayName(row, group.type) })}
                      onclick={() => downloadAction(row.download!)}
                    />
                  </span>
                {:else if update}
                  <span class="download-action">
                    <IconButton
                      icon="i-mdi-download"
                      label={m.data_update_file({ name: displayName(row, group.type) })}
                      disabled={updatePending || downloads === null || downloadError}
                      onclick={() => startUpdates([update.path])}
                    />
                  </span>
                {/if}
              </li>
            {/each}
          </ul>
        </ResponsiveCard>
      </section>
    {/each}
  </ScreenScaffold>
</div>

<Dialog.Root bind:open={detailsOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="data-dialog-overlay" />
    <Dialog.Content class="data-dialog-content" onCloseAutoFocus={restoreFocus}>
      {#if selectedSource && selectedGroup}
        <header>
          <span aria-hidden="true" class={[selectedGroup.icon, 'type-icon']}></span>
          <div class="description">
            <Dialog.Title class="data-dialog-title" level={2}
              >{displayName(selectedSource, selectedGroup.type)}</Dialog.Title
            >
            <Dialog.Description class="data-dialog-description"
              >{selectedGroup.label}</Dialog.Description
            >
          </div>
          <Dialog.Close class="data-dialog-close" aria-label={m.data_close()}>×</Dialog.Close>
        </header>
        <label class="activation">
          <span>
            <strong>{m.data_enabled()}</strong>
            <span id={`${componentId}-enabled-hint`} class="activation-hint"
              >{m.data_enabled_hint()}</span
            >
          </span>
          <span class="activation-control">
            <input
              type="checkbox"
              role="switch"
              aria-label={m.data_enabled()}
              aria-describedby={`${componentId}-enabled-hint`}
              checked={selectedEnabled}
              onchange={(event) => {
                let enabled = event.currentTarget.checked;
                event.currentTarget.checked = !!selectedEnabled;
                activation.setEnabled(selectedGroup.type, selectedSource.sourceName, enabled);
              }}
            />
            <span class="activation-check" aria-hidden="true"
              ><span class="i-mdi-check-bold"></span></span
            >
          </span>
        </label>
        {#if activation.hasError(selectedGroup.type, selectedSource.sourceName)}
          <p class="error" role="alert">{m.data_activation_failed()}</p>
        {/if}
        {#if selectedGroup.type === 'basemap'}
          <dl>
            <div>
              <dt>{m.data_source()}</dt>
              <dd>Enroute</dd>
            </div>
            {#if fileDetails}
              <div>
                <dt>{m.data_file_size()}</dt>
                <dd>{size(fileDetails.size)}</dd>
              </div>
              <div>
                <dt>{m.data_downloaded_at()}</dt>
                <dd>
                  {new Intl.DateTimeFormat(getLocale(), {
                    dateStyle: 'medium',
                    timeStyle: 'short',
                  }).format(fileDetails.modifiedAt)}
                </dd>
              </div>
            {/if}
          </dl>
          {#if fileDetailsError}
            <p class="error" role="alert">{m.data_file_details_failed()}</p>
            <Button variant="secondary" onclick={() => fileDetailsRetry++}>{m.retry()}</Button>
          {:else if !fileDetails}<p role="status">{m.data_file_details_loading()}</p>{/if}
        {:else if selectedGroup.type === 'airspace' || selectedGroup.type === 'waypoints'}
          <dl>
            <div>
              <dt>{m.data_source()}</dt>
              <dd>{m.data_imported()}</dd>
            </div>
            {#if selectedEnabled && selectedSource.type === 'active' && ('airspaceCount' in selectedSource || 'waypointCount' in selectedSource)}
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
        {/if}
        {#if selectedEnabled && selectedSource.type === 'unavailable'}
          <p class="error">{metadata(selectedSource, selectedGroup.type)}</p>
        {:else if selectedEnabled && selectedSource.type === 'active' && 'waypointCount' in selectedSource && selectedSource.warnings.length}
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
        {#if downloadActionError}<p class="error" role="alert">{downloadActionError}</p>{/if}
        {#if selectedDownload}<p role="status">{downloadStatus(selectedDownload)}</p>{/if}
        {#if selectedDownloadEntry}
          <Button
            loading={updatePending}
            disabled={downloads === null ||
              downloadError ||
              (selectedDownload && selectedDownload.type !== 'failed')}
            size="large"
            style="width: 100%; margin-block-end: var(--space-4)"
            onclick={() => startUpdates([selectedDownloadEntry.path])}
            >{selectedSource.type === 'unavailable'
              ? m.data_download_again()
              : m.data_update()}</Button
          >
        {/if}
        <Button
          disabled={activation.pending}
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

{#if dataImport.selection}
  <ConfirmDialog
    open={true}
    title={m.data_replace()}
    description={m.data_replace_description({ name: dataImport.selection.sourceName })}
    cancelLabel={m.cancel()}
    confirmLabel={m.data_replace_confirm()}
    onCancel={() => dataImport.cancelImport()}
    onConfirm={async () => {
      if (dataImport.selection) await dataImport.importFile(dataImport.selection);
      await returnAfterImport();
    }}
  />
{/if}

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
    min-width: 0;
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
  .files > li {
    display: flex;
    align-items: center;
  }
  div.file-row {
    cursor: default;
  }
  .download-action {
    padding-inline-end: calc(var(--space-5) + var(--card-safe-area-end));
  }
  .download-status {
    display: block;
    margin-block-start: var(--space-1);
    color: var(--color-text-muted);
    font: var(--text-row-detail);
  }
  .download-status.active {
    color: var(--color-action-primary-surface);
  }
  .download-failed .download-status,
  .download-failed .type-icon {
    color: var(--color-error-text);
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
  button.file-row:active {
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
  .activation {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-block: var(--space-4);
    padding-block: var(--space-4);
    border-block: 1px solid var(--color-separator);
    cursor: pointer;
  }
  .activation > span:first-child {
    flex: 1;
  }
  .activation strong {
    font: var(--text-row-label);
  }
  .activation-hint {
    display: block;
    color: var(--color-text-muted);
  }
  .activation-control {
    display: grid;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
  }
  .activation input {
    grid-area: 1 / 1;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }
  .activation-check {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px solid var(--color-border-strong);
    border-radius: 3px;
    background: var(--color-card-surface);
    color: var(--color-white);
    pointer-events: none;
  }
  .activation-check > span {
    font-size: 20px;
    opacity: 0;
  }
  .activation input:checked + .activation-check {
    border-color: var(--color-action-primary-surface);
    background: var(--color-action-primary-surface);
  }
  .activation input:checked + .activation-check > span {
    opacity: 1;
  }
  .activation input:focus-visible + .activation-check {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }
  @media (min-width: 545px) {
    dl {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
</style>

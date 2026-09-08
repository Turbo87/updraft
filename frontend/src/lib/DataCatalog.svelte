<script lang="ts">
  import type { EnrouteCatalogStatus } from './client';

  import Button from './Button.svelte';
  import ListRow from './ListRow.svelte';
  import { m } from './paraglide/messages.js';
  import { getLocale } from './paraglide/runtime.js';
  import ResponsiveCard from './ResponsiveCard.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    status: EnrouteCatalogStatus | null;
    onCountry: (countryCode: string) => void;
    onImport: (opener: HTMLButtonElement) => void;
    onBack: () => void;
    country?: string;
    importPending?: boolean;
    importError?: string;
    subscriptionError?: boolean;
    onRetry: () => void | Promise<void>;
  };

  let {
    status,
    onCountry,
    onImport,
    onRetry,
    onBack,
    country,
    importPending = false,
    importError = '',
    subscriptionError = false,
  }: Props = $props();
  let retryPending = $state(false);
  let retryError = $state(false);
  let entries = $derived(
    status?.cached?.entries.filter((entry) => entry.countryCode === country) ?? [],
  );
  let countryName = $derived(
    country ? new Intl.DisplayNames(getLocale(), { type: 'region' }).of(country) : undefined,
  );
  async function retry() {
    retryPending = true;
    retryError = false;
    try {
      await onRetry();
    } catch {
      retryError = true;
    } finally {
      retryPending = false;
    }
  }
  const componentId = $props.id();
  let groups = $derived.by(() => {
    let names = new Intl.DisplayNames(getLocale(), { type: 'region' });
    let collator = new Intl.Collator(getLocale());
    let continents = {
      africa: m.data_africa(),
      asia: m.data_asia(),
      oceania: m.data_oceania(),
      europe: m.data_europe(),
      northAmerica: m.data_north_america(),
      southAmerica: m.data_south_america(),
    };
    return Object.entries(continents)
      .map(([continent, label]) => {
        let codes = new Set(
          status?.cached?.entries
            .filter((entry) => entry.continent === continent)
            .map((entry) => entry.countryCode),
        );
        let countries = Array.from(codes, (code) => ({ code, name: names.of(code)! }));
        countries.sort((a, b) => collator.compare(a.name, b.name));
        return { continent, label, countries };
      })
      .filter((group) => group.countries.length > 0);
  });
</script>

{#snippet importAction()}
  <Button
    loading={importPending}
    onclick={(event) => onImport(event.currentTarget)}
    style="width: 100%">{m.data_import_custom()}</Button
  >
{/snippet}

<ScreenScaffold
  {onBack}
  backLabel={country ? m.back_to_add_data() : m.back_to_data()}
  title={countryName ?? m.data_add()}
  responsiveActions
  actions={country ? undefined : importAction}
>
  {#if importError}<p role="alert">{importError}</p>{/if}
  {#if retryError}<p role="alert">{m.data_catalog_refresh_failed()}</p>{/if}
  {#if subscriptionError}<p role="alert">{m.data_catalog_subscription_failed()}</p>
  {:else if status?.error || (country ? entries.length === 0 : groups.length === 0)}
    <ResponsiveCard>
      <div class="notice" role="status">
        {#if status?.error}
          <strong
            >{status.cached ? m.data_enroute_unreachable() : m.data_catalog_unavailable()}</strong
          >
          {#if status.cached}
            <p>
              {m.data_catalog_last_checked({
                date: new Intl.DateTimeFormat(getLocale(), {
                  dateStyle: 'medium',
                  timeStyle: 'short',
                }).format(status.cached.checkedAt),
              })}
            </p>
          {:else}
            <p>{m.data_enroute_unreachable()}</p>
            <p>{m.data_catalog_unavailable_hint()}</p>
          {/if}
          <Button variant="secondary" disabled={status.refreshing || retryPending} onclick={retry}
            >{m.retry()}</Button
          >
        {:else if status === null || status.refreshing}
          <p>{m.data_catalog_loading()}</p>
        {:else}
          <strong>{m.data_catalog_unavailable()}</strong>
          <p>{m.data_catalog_unavailable_hint()}</p>
        {/if}
      </div>
    </ResponsiveCard>
  {/if}
  {#if country}
    {#if entries.length > 0}
      <section aria-labelledby={`${componentId}-basemaps`}>
        <h2 id={`${componentId}-basemaps`}>{m.data_basemap()}</h2>
        <ResponsiveCard>
          {#each entries as entry (entry.path)}
            <ListRow
              label={entries.length === 1
                ? countryName!
                : entry.path
                    .split('/')
                    .at(-1)!
                    .replace(/\.mbtiles$/, '')}
              value={`${new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeZone: 'UTC' }).format(new Date(entry.publicationDate))} · ${new Intl.NumberFormat(getLocale(), { style: 'unit', unit: 'megabyte', maximumFractionDigits: 1 }).format(entry.size / 1_000_000)}`}
            />
          {/each}
        </ResponsiveCard>
      </section>
    {/if}
  {:else}
    {#each groups as group (group.continent)}
      <section aria-labelledby={`${componentId}-${group.continent}`}>
        <h2 id={`${componentId}-${group.continent}`}>{group.label}</h2>
        <ResponsiveCard>
          <ul>
            {#each group.countries as country (country.code)}
              <li>
                <button
                  class="country"
                  disabled={importPending}
                  onclick={() => onCountry(country.code)}
                >
                  <span
                    aria-hidden="true"
                    class={`flag i-circle-flags-${country.code.toLowerCase()}`}
                  ></span>
                  <span class="name">{country.name}</span>
                  <span aria-hidden="true" class="i-mdi-chevron-right"></span>
                </button>
              </li>
            {/each}
          </ul>
        </ResponsiveCard>
      </section>
    {/each}
  {/if}
</ScreenScaffold>

<style>
  section {
    margin-block-end: var(--space-4);
  }
  h2 {
    margin: var(--space-4) var(--space-1);
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
  li + li {
    border-block-start: 1px solid var(--color-separator);
  }
  section :global(.list-row + .list-row) {
    border-block-start: 1px solid var(--color-separator);
  }
  section :global(.list-row) {
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
  }
  .country {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    width: 100%;
    min-height: var(--target-min);
    padding-block: var(--space-4);
    padding-inline: calc(var(--space-6) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
    border: 0;
    background: transparent;
    color: var(--color-text);
    font: var(--text-row-label);
    text-align: start;
    cursor: pointer;
  }
  .country:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }
  .country:active {
    background: var(--color-control-surface-pressed);
  }
  .country > span:not(.name) {
    flex: 0 0 auto;
    font-size: 24px;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .notice {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: var(--space-4);
    padding-block: var(--space-4);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
    font: var(--text-body);
  }
  .notice > :not(:global(button)) {
    grid-column: 1;
  }
  .notice :global(button) {
    grid-column: 2;
    grid-row: 1 / span 3;
    align-self: center;
  }
  p {
    margin: var(--space-2) 0 0;
    color: var(--color-text-muted);
  }
</style>

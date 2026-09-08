<script lang="ts">
  import type { AirspaceStatus } from './protocol/generated/AirspaceStatus';
  import type { WaypointStatus } from './protocol/generated/WaypointStatus';

  import { m } from './paraglide/messages.js';
  import { getLocale } from './paraglide/runtime.js';
  import ResponsiveCard from './ResponsiveCard.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import StatusPill from './StatusPill.svelte';

  type Props = { airspace: AirspaceStatus; waypoints: WaypointStatus };
  type Source = AirspaceStatus['sources'][number] | WaypointStatus['sources'][number];

  let { airspace, waypoints }: Props = $props();
  const groups = $derived(
    [
      {
        type: 'airspace',
        label: m.airspace_label(),
        icon: 'i-mdi-vector-square',
        sources: airspace.sources,
      },
      {
        type: 'waypoints',
        label: m.waypoints_heading(),
        icon: 'i-mdi-map-marker',
        sources: waypoints.sources,
      },
    ].filter((group) => group.sources.length > 0),
  );

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
        <ul>
          {#each group.sources.toSorted(compareSources) as source (source.sourceName)}
            <li
              class:disabled={source.type === 'disabled'}
              class:unavailable={source.type === 'unavailable'}
            >
              <span aria-hidden="true" class={[group.icon, 'type-icon']}></span>
              <div class="description">
                <div class="name">
                  <h3>{source.sourceName}</h3>
                  {#if source.type === 'disabled'}<StatusPill label={m.data_disabled()} />{/if}
                </div>
                <p class="metadata">{metadata(source)}</p>
              </div>
            </li>
          {/each}
        </ul>
      </ResponsiveCard>
    </section>
  {/each}
</ScreenScaffold>

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
  li {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    min-height: var(--target-min);
    padding-block: var(--space-4);
    padding-inline: calc(var(--space-6) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
  }
  li + li {
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
  h3 {
    margin: 0;
    overflow-wrap: anywhere;
    font: var(--text-row-label);
  }
  .metadata {
    margin: var(--space-1) 0 0;
    color: var(--color-text-muted);
    font: 400 15px/1.4 var(--font-ui);
  }
  .disabled h3 {
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
</style>

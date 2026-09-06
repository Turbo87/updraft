<script lang="ts">
  import type { Map, MapEventType, MapGeoJSONFeature } from 'maplibre-gl';
  import type { AirspaceProperties } from '$lib/airspace';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { Locale } from '$lib/protocol/generated/Locale';
  import type { AirspaceStore } from '$lib/stores/airspace.svelte';

  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';

  import { m } from '$lib/paraglide/messages.js';

  type QueryState = { type: 'loading' } | { type: 'ready'; features: MapGeoJSONFeature[] };

  let {
    airspace,
    locale,
    map,
    position,
  }: { airspace: AirspaceStore; locale: Locale; map: Map; position: LatLon } = $props();
  let queryState = $state.raw<QueryState>({ type: 'loading' });

  function queryAirspaces() {
    if (queryState.type === 'ready') return;

    if (!airspace.current.sources.some((source) => source.type === 'active')) {
      queryState = { type: 'ready', features: [] };
      return;
    }

    if (
      !map.isStyleLoaded() ||
      !map.getSource('airspace') ||
      !map.isSourceLoaded('airspace') ||
      !map.getLayer('airspace-hit')
    ) {
      return;
    }

    let point = map.project([position.longitudeDegrees, position.latitudeDegrees]);
    queryState = {
      type: 'ready',
      features: map
        .queryRenderedFeatures(point, { layers: ['airspace-hit'] })
        .filter((feature) => String(feature.id).startsWith(`${airspace.current.generation}:`)),
    };
  }

  function handleSourceData(event: MapEventType['sourcedata']) {
    if (event.sourceId !== 'airspace') return;
    queryState = { type: 'loading' };
    queryAirspaces();
  }

  function handleMapError(event: MapEventType['error']) {
    if ('sourceId' in event && event.sourceId === 'airspace') {
      queryState = { type: 'ready', features: [] };
    }
  }

  onMount(() => {
    map.on('styledata', queryAirspaces);
    map.on('sourcedata', handleSourceData);
    map.on('error', handleMapError);
    queryAirspaces();

    return () => {
      map.off('styledata', queryAirspaces);
      map.off('sourcedata', handleSourceData);
      map.off('error', handleMapError);
    };
  });

  function airspaceName(feature: MapGeoJSONFeature): string {
    let name = feature.properties.name;
    return typeof name === 'string' && name !== '' ? name : m.unnamed_airspace();
  }

  function airspaceDetail(feature: MapGeoJSONFeature): string {
    let properties = feature.properties as AirspaceProperties;
    let type = m.airspace_type_value({ type: properties.type }, { locale });
    return properties.icaoClass === 8
      ? type
      : `${type} · ${m.icao_class_value({ icaoClass: properties.icaoClass }, { locale })}`;
  }
</script>

{#if queryState.type === 'loading'}
  <p class="empty-results">{m.loading_nearby_airspaces()}</p>
{:else if queryState.features.length === 0}
  <p class="empty-results">{m.no_nearby_airspaces()}</p>
{:else}
  <ul class="result-list">
    {#each queryState.features as feature (feature)}
      <li>
        <a href={resolve('/airspaces/[id]', { id: String(feature.id) })}>
          <span class="text">
            <span class="name">{airspaceName(feature)}</span>
            <span class="detail">{airspaceDetail(feature)}</span>
          </span>
          <span aria-hidden="true" class="i-mdi-chevron-right chevron"></span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .result-list {
    box-sizing: border-box;
    display: grid;
    width: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    list-style: none;
  }

  li + li {
    border-block-start: 1px solid var(--color-separator);
  }

  li {
    min-width: 0;
  }

  a {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-width: 0;
    min-height: var(--target-flight);
    padding: var(--space-2) var(--space-4) var(--space-2) var(--space-5);
    border: 0;
    border-radius: 0;
    color: var(--color-text);
    text-decoration: none;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  a:active {
    background: var(--color-control-surface-pressed);
  }

  a:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }

  .text {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
  }

  .name,
  .detail {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font: var(--text-row-label);
  }

  .detail {
    color: var(--color-text-muted);
    font: var(--text-row-detail);
  }

  .chevron {
    flex: 0 0 auto;
    margin-inline-start: auto;
    color: var(--color-text-muted);
    font-size: 1.75rem;
    line-height: 1;
  }

  .empty-results {
    margin: 0;
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  @media (prefers-reduced-motion: reduce) {
    a {
      transition: none;
    }
  }
</style>

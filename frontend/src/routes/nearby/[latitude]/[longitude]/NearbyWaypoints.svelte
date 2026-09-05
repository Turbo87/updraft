<script lang="ts">
  import type { MapGeoJSONFeature, Map as MapLibreMap } from 'maplibre-gl';
  import type { MapState } from '$lib/map-state.svelte';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { AltitudeUnit } from '$lib/units';

  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';

  import { m } from '$lib/paraglide/messages.js';
  import { convertAltitude } from '$lib/units';
  import WaypointSymbol from '$lib/WaypointSymbol.svelte';

  let {
    map,
    position,
    sourceStatus,
    altitudeUnit,
  }: {
    map: MapLibreMap;
    position: LatLon;
    sourceStatus: MapState['waypointSourceStatus'];
    altitudeUnit: AltitudeUnit;
  } = $props();
  let features = $state.raw<MapGeoJSONFeature[] | null>(null);

  function query() {
    if (
      sourceStatus !== 'ready' ||
      !map.getLayer('waypoint-hit') ||
      !map.isSourceLoaded('waypoints')
    )
      return;
    let point = map.project([position.longitudeDegrees, position.latitudeDegrees]);
    let hits = map.queryRenderedFeatures(point, { layers: ['waypoint-hit'] });
    features = [...new Map(hits.map((feature) => [feature.properties.id, feature])).values()];
  }
  $effect(query);
  onMount(() => {
    map.on('idle', query);
    return () => {
      map.off('idle', query);
    };
  });
</script>

{#if sourceStatus === 'failed'}
  <p role="alert">{m.waypoint_load_failed()}</p>
{:else if sourceStatus === 'loading' || features === null}
  <p class="empty-results">{m.waypoint_loading()}</p>
{:else if features.length === 0}
  <p class="empty-results">{m.waypoint_none_nearby()}</p>
{:else}
  <ul>
    {#each features as feature (feature.properties.id)}
      <li>
        <a href={resolve('/waypoints/[id]', { id: String(feature.properties.id) })}>
          <WaypointSymbol
            kind={feature.properties.kind}
            runwayDirection={feature.properties.runwayDirection}
          />
          <span class="text">
            <span class="name">{feature.properties.name}</span>
            <span class="detail">
              {[
                `${convertAltitude(feature.properties.elevationMeters, altitudeUnit).toFixed(0)} ${altitudeUnit}`,
                feature.properties.frequency && `${feature.properties.frequency} MHz`,
                feature.properties.notes ||
                  m.waypoint_type_value({ kind: feature.properties.kind }),
              ]
                .filter(Boolean)
                .join(' · ')}
            </span>
          </span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .empty-results {
    margin: 0;
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    overflow: hidden;
  }
  li + li {
    border-block-start: 1px solid var(--color-separator);
  }
  a {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    --waypoint-symbol-size: 1.4rem;
    min-height: var(--target-flight);
    padding: var(--space-2) var(--space-4);
    color: var(--color-text);
    text-decoration: none;
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  a:active {
    background: var(--color-control-surface-pressed);
  }
  .name,
  .detail {
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
</style>

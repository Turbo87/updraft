<script lang="ts">
  import type { MapGeoJSONFeature, Map as MapLibreMap } from 'maplibre-gl';
  import type { MapState } from '$lib/map-state.svelte';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { AltitudeUnit } from '$lib/units';

  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';

  import { m } from '$lib/paraglide/messages.js';
  import ResponsiveCard from '$lib/ResponsiveCard.svelte';
  import { convertAltitude } from '$lib/units';
  import WaypointSymbol from '$lib/WaypointSymbol.svelte';

  type Props = {
    map: MapLibreMap;
    position: LatLon;
    sourceStatus: MapState['waypointSourceStatus'];
    altitudeUnit: AltitudeUnit;
  };

  let { map, position, sourceStatus, altitudeUnit }: Props = $props();
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
  <ResponsiveCard>
    <p class="empty-results">{m.waypoint_loading()}</p>
  </ResponsiveCard>
{:else if features.length === 0}
  <ResponsiveCard>
    <p class="empty-results">{m.waypoint_none_nearby()}</p>
  </ResponsiveCard>
{:else}
  <ResponsiveCard>
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
  </ResponsiveCard>
{/if}

<style>
  .empty-results {
    margin: 0;
    padding-block: var(--space-5);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
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
    padding-block: var(--space-2);
    padding-inline: calc(var(--space-4) + var(--card-safe-area-start))
      calc(var(--space-4) + var(--card-safe-area-end));
    color: var(--color-text);
    text-decoration: none;
  }
  a:focus-visible {
    outline-offset: -2px;
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

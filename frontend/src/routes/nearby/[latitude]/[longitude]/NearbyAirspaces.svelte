<script lang="ts">
  import type { Map, MapEventType, MapGeoJSONFeature } from 'maplibre-gl';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { AirspaceStore } from '$lib/stores/airspace.svelte';

  import { onMount } from 'svelte';

  import { m } from '$lib/paraglide/messages.js';

  type QueryState = { type: 'loading' } | { type: 'ready'; features: MapGeoJSONFeature[] };

  let { airspace, map, position }: { airspace: AirspaceStore; map: Map; position: LatLon } =
    $props();
  let queryState = $state.raw<QueryState>({ type: 'loading' });

  function queryAirspaces() {
    if (queryState.type === 'ready') return;

    if (airspace.current.type !== 'active') {
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
      features: map.queryRenderedFeatures(point, { layers: ['airspace-hit'] }),
    };
  }

  function handleMapError(event: MapEventType['error']) {
    if ('sourceId' in event && event.sourceId === 'airspace') {
      queryState = { type: 'ready', features: [] };
    }
  }

  onMount(() => {
    map.on('styledata', queryAirspaces);
    map.on('sourcedata', queryAirspaces);
    map.on('error', handleMapError);
    queryAirspaces();

    return () => {
      map.off('styledata', queryAirspaces);
      map.off('sourcedata', queryAirspaces);
      map.off('error', handleMapError);
    };
  });

  function airspaceName(feature: MapGeoJSONFeature): string {
    let name = feature.properties.name;
    return typeof name === 'string' && name !== '' ? name : m.unnamed_airspace();
  }
</script>

{#if queryState.type === 'loading'}
  <p>{m.loading_nearby_airspaces()}</p>
{:else if queryState.features.length === 0}
  <p>{m.no_nearby_airspaces()}</p>
{:else}
  <ul>
    {#each queryState.features as feature (feature)}
      <li>{airspaceName(feature)}</li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    margin: 0;
    padding-inline-start: 1.5rem;
  }
</style>

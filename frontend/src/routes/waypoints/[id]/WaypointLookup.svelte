<script lang="ts">
  import type { FeatureCollection, Point } from 'geojson';
  import type { GeoJSONSource, Map, MapEventType } from 'maplibre-gl';
  import type { MapState } from '$lib/map-state.svelte';
  import type { AltitudeUnit } from '$lib/units';
  import type { WaypointFeature, WaypointProperties } from '$lib/waypoints';

  import Button from '$lib/Button.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import WaypointDetails from '$lib/WaypointDetails.svelte';

  let {
    map,
    id,
    generation,
    altitudeUnit,
    sourceStatus,
    onBack,
  }: {
    map: Map;
    id: string;
    generation: number;
    altitudeUnit: AltitudeUnit;
    sourceStatus: MapState['waypointSourceStatus'];
    onBack: () => void;
  } = $props();
  type State =
    { type: 'loading' | 'failed' | 'notFound' } | { type: 'ready'; waypoint: WaypointFeature };
  let queryState = $state.raw<State>({ type: 'loading' });
  let retryCount = $state(0);

  function retry() {
    retryCount++;
    let source = map.getSource<GeoJSONSource>('waypoints');
    let data = source?.serialize().data;
    if (source && data) void source.setData(data);
  }

  $effect(() => {
    let currentId = id;
    let currentMap = map;
    let currentGeneration = generation;
    void retryCount;
    let active = true;
    let querying = false;
    queryState = { type: 'loading' };
    if (!/^\d+:\d+:\d+$/.test(currentId) || currentId.split(':')[0] !== String(currentGeneration)) {
      queryState = { type: 'notFound' };
      return;
    }
    if (sourceStatus === 'failed') {
      queryState = { type: 'failed' };
      return;
    }
    async function query() {
      if (querying) return;
      let source = currentMap.getSource<GeoJSONSource>('waypoints');
      if (!source || !currentMap.isSourceLoaded('waypoints')) return;
      querying = true;
      try {
        let data = (await source.getData()) as FeatureCollection<Point, WaypointProperties>;
        if (!active) return;
        let waypoint = data.features.find((feature) => feature.properties.id === currentId);
        queryState = waypoint ? { type: 'ready', waypoint } : { type: 'notFound' };
      } catch {
        if (active) queryState = { type: 'failed' };
      }
    }
    function handleError(event: MapEventType['error']) {
      if ('sourceId' in event && event.sourceId === 'waypoints') {
        active = false;
        queryState = { type: 'failed' };
      }
    }
    currentMap.on('sourcedata', query);
    currentMap.on('styledata', query);
    currentMap.on('error', handleError);
    void query();
    return () => {
      active = false;
      currentMap.off('sourcedata', query);
      currentMap.off('styledata', query);
      currentMap.off('error', handleError);
    };
  });
</script>

{#if queryState.type === 'ready'}
  <WaypointDetails waypoint={queryState.waypoint} {altitudeUnit} {onBack} />
{:else}
  <ScreenScaffold title={m.waypoints_heading()} backLabel={m.waypoint_back()} {onBack}>
    {#if queryState.type === 'failed'}
      <p role="alert">{m.waypoint_load_failed()}</p>
      <Button onclick={retry}>{m.retry()}</Button>
    {:else}
      <p>{queryState.type === 'loading' ? m.waypoint_loading() : m.waypoint_not_found()}</p>
    {/if}
  </ScreenScaffold>
{/if}

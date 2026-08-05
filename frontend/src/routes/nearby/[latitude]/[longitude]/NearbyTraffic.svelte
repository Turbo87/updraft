<script lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { Locale } from '$lib/protocol/generated/Locale';
  import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
  import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';
  import type { RetainedTraffic } from './nearby-traffic';

  import { onMount } from 'svelte';

  import { m } from '$lib/paraglide/messages.js';
  import {
    createRetainedTraffic,
    formatTrafficId,
    formatTrafficType,
    refreshRetainedTraffic,
  } from './nearby-traffic';

  type QueryState = { type: 'loading' } | { type: 'ready'; traffic: RetainedTraffic[] };

  let {
    locale,
    map,
    position,
    traffic,
  }: { locale: Locale; map: Map; position: LatLon; traffic: TrafficStore } = $props();
  let queryState = $state.raw<QueryState>({ type: 'loading' });

  function queryTraffic() {
    if (queryState.type === 'ready') return;

    if (
      !map.isStyleLoaded() ||
      !map.getSource('traffic') ||
      !map.isSourceLoaded('traffic') ||
      !map.getLayer('traffic-hit')
    ) {
      return;
    }

    let point = map.project([position.longitudeDegrees, position.latitudeDegrees]);
    let ids = map.queryRenderedFeatures(point, { layers: ['traffic-hit'] }).map(({ id }) => {
      if (typeof id !== 'string') throw new TypeError('Traffic hit feature has no string ID');
      return id;
    });
    queryState = { type: 'ready', traffic: createRetainedTraffic(ids, traffic.current) };
  }

  function handleTrafficUpdate(
    _update: TrafficUpdate,
    currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
  ) {
    if (queryState.type === 'loading') return;
    queryState = {
      type: 'ready',
      traffic: refreshRetainedTraffic(queryState.traffic, currentTargets),
    };
  }

  function trafficLabel(retained: RetainedTraffic): string {
    let label = retained.target
      ? `${formatTrafficType(retained.target.trafficType, locale)} · ${formatTrafficId(retained.id)}`
      : formatTrafficId(retained.id);
    return retained.available ? label : `${label} · ${m.unavailable_value()}`;
  }

  onMount(() => {
    map.on('styledata', queryTraffic);
    map.on('sourcedata', queryTraffic);
    let unsubscribe = traffic.subscribe(handleTrafficUpdate);
    queryTraffic();

    return () => {
      map.off('styledata', queryTraffic);
      map.off('sourcedata', queryTraffic);
      unsubscribe();
    };
  });
</script>

{#if queryState.type === 'loading'}
  <p>{m.loading_nearby_traffic()}</p>
{:else if queryState.traffic.length === 0}
  <p>{m.no_nearby_traffic()}</p>
{:else}
  <ul>
    {#each queryState.traffic as retained (retained)}
      <li>{trafficLabel(retained)}</li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    margin: 0;
    padding-inline-start: 1.5rem;
  }
</style>

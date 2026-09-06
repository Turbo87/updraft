<script lang="ts">
  import type { Map, MapEventType } from 'maplibre-gl';
  import type { ArrivalViewport, UpdraftClient } from '$lib/client';
  import type { AltitudeUnit } from '$lib/units';

  import { GeoJSONSource } from 'svelte-maplibre-gl';

  import WaypointLayers from './WaypointLayers.svelte';

  type Props = {
    client: UpdraftClient;
    map: Map;
    generation: number;
    altitudeUnit: AltitudeUnit;
    onReady: (ready: boolean) => void;
  };

  let { client, map, generation, altitudeUnit, onReady }: Props = $props();
  let data = $state<string | null>(null);
  const subscriptionClient = $derived(client);
  const subscriptionMap = $derived(map);
  const catalogGeneration = $derived(generation);
  const readinessListener = $derived(onReady);

  $effect(() => {
    let currentMap = subscriptionMap;
    let currentGeneration = catalogGeneration;
    let active = true;
    let reportReady = readinessListener;
    reportReady(false);
    data = null;
    function bounds(): ArrivalViewport {
      let bounds = currentMap.getBounds();
      return [bounds.getWest(), bounds.getSouth(), bounds.getEast(), bounds.getNorth()];
    }
    let subscription = subscriptionClient.subscribeArrivals(
      bounds(),
      (update) => {
        if (active && update.generation === currentGeneration) data = update.url;
      },
      (error) => {
        if (!active) return;
        data = null;
        reportReady(false);
        console.error('Arrival subscription failed', error);
      },
    );
    function updateViewport() {
      void subscription.updateViewport(bounds()).catch((error: unknown) => {
        console.error('Failed to update arrival viewport', error);
      });
    }
    currentMap.on('move', updateViewport);
    function sourceLoaded(event: MapEventType['sourcedata']) {
      if (!active || !data) return;
      if (event.sourceId === 'arrivals' && event.sourceDataType === 'content') reportReady(true);
    }
    currentMap.on('sourcedata', sourceLoaded);
    return () => {
      active = false;
      currentMap.off('move', updateViewport);
      currentMap.off('sourcedata', sourceLoaded);
      reportReady(false);
      void subscription.close().catch((error: unknown) => {
        console.error('Failed to close arrival subscription', error);
      });
    };
  });
</script>

{#if data}
  <GeoJSONSource id="arrivals" {data}>
    <WaypointLayers
      id="arrival"
      arrivalUnit={altitudeUnit}
      filter={['==', ['get', 'catalogGeneration'], generation]}
    />
  </GeoJSONSource>
{/if}

<script lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { ArrivalViewport, UpdraftClient } from '$lib/client';

  import { GeoJSONSource } from 'svelte-maplibre-gl';

  let { client, map, generation }: { client: UpdraftClient; map: Map; generation: number } =
    $props();
  let data = $state<string | null>(null);

  $effect(() => {
    let currentMap = map;
    let currentGeneration = generation;
    let active = true;
    data = null;
    function bounds(): ArrivalViewport {
      let bounds = currentMap.getBounds();
      return [bounds.getWest(), bounds.getSouth(), bounds.getEast(), bounds.getNorth()];
    }
    let subscription = client.subscribeArrivals(
      bounds(),
      (update) => {
        if (active && update.generation === currentGeneration) data = update.url;
      },
      (error) => {
        if (!active) return;
        data = null;
        console.error('Arrival subscription failed', error);
      },
    );
    function updateViewport() {
      void subscription.updateViewport(bounds()).catch((error: unknown) => {
        console.error('Failed to update arrival viewport', error);
      });
    }
    currentMap.on('move', updateViewport);
    return () => {
      active = false;
      currentMap.off('move', updateViewport);
      void subscription.close().catch((error: unknown) => {
        console.error('Failed to close arrival subscription', error);
      });
    };
  });
</script>

{#if data}
  <GeoJSONSource id="arrivals" {data} />
{/if}

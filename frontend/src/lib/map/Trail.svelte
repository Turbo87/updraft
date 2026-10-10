<script lang="ts">
  import type { GeoJSONSource as MapLibreGeoJSONSource } from 'maplibre-gl';
  import type { TrailStore } from '#lib/stores/trail.js';

  import { onMount } from 'svelte';
  import { GeoJSONSource, LineLayer } from 'svelte-maplibre-gl';

  import { TRAIL_COLOR } from './trail';

  type Props = {
    trail: TrailStore;
  };

  let { trail }: Props = $props();

  let source: MapLibreGeoJSONSource | undefined = $state();
  let updateQueue = Promise.resolve();

  // The source misses the diffs that arrive before it exists.
  $effect(() => {
    let activeSource = source;
    if (!activeSource) return;

    let data = trail.segments.featureCollection();
    enqueue(activeSource, () => activeSource.setData(data));
  });

  onMount(() =>
    trail.subscribe((diff) => {
      let activeSource = source;
      if (!activeSource) return;

      enqueue(activeSource, () => activeSource.updateData(diff));
    }),
  );

  function enqueue(activeSource: MapLibreGeoJSONSource, update: () => Promise<void>) {
    updateQueue = updateQueue
      .then(update)
      .catch((error: unknown) => {
        console.warn('Trail source update failed. Rebuilding the source.', error);
        return activeSource.setData(trail.segments.featureCollection());
      })
      .catch((error: unknown) => {
        console.warn('Trail source rebuild failed.', error);
      });
  }
</script>

<!-- Simplification drops whole lines that are shorter than `tolerance`. -->
<GeoJSONSource
  id="trail"
  maxzoom={24}
  tolerance={0}
  data={{ type: 'FeatureCollection', features: [] }}
  bind:source
>
  <LineLayer
    id="trail"
    layout={{ 'line-cap': 'round', 'line-join': 'round' }}
    paint={{ 'line-color': TRAIL_COLOR, 'line-width': 3 }}
  />
</GeoJSONSource>

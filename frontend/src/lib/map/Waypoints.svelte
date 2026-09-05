<script lang="ts">
  import type { GeoJSONSourceSpecification } from 'maplibre-gl';

  import { CircleLayer, GeoJSONSource } from 'svelte-maplibre-gl';

  import WaypointLayers from './WaypointLayers.svelte';

  let {
    data,
    showHitAreas,
    showLandables = true,
  }: {
    data: GeoJSONSourceSpecification['data'];
    showHitAreas: boolean;
    showLandables?: boolean;
  } = $props();
</script>

<GeoJSONSource id="waypoints" {data}>
  <CircleLayer
    id="waypoint-hit"
    beforeId="traffic-fixed"
    paint={{ 'circle-radius': 24, 'circle-opacity': showHitAreas ? 0.2 : 0 }}
  />
  <WaypointLayers {showLandables} />
</GeoJSONSource>

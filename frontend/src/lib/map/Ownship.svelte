<script lang="ts">
  import type { Availability } from '$lib/protocol/generated/Availability';
  import type { LatLon } from '$lib/protocol/generated/LatLon';

  import { GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { ownshipFeature } from './ownship';

  let { position, track }: { position: LatLon; track: Availability<number> } = $props();
</script>

<GeoJSONSource id="ownship" maxzoom={24} data={ownshipFeature(position, track)}>
  <SymbolLayer
    id="ownship-symbol"
    layout={{
      'icon-image': 'updraft-sdf:glider',
      'icon-rotation-alignment': 'map',
      'icon-rotate': ['get', 'track'],
      'icon-allow-overlap': true,
    }}
    paint={{
      'icon-color': '#2d55a6',
      'icon-halo-color': '#ffffff',
      'icon-halo-width': 2,
      'icon-halo-blur': 0.5,
    }}
  />
</GeoJSONSource>

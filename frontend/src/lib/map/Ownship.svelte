<script lang="ts">
  import type { LatLon } from '$lib/protocol/generated/LatLon';

  import { GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { ownshipFeature } from './ownship';

  let { position, trackDegrees }: { position: LatLon; trackDegrees: number | null } = $props();
</script>

<GeoJSONSource id="ownship" maxzoom={24} data={ownshipFeature(position, trackDegrees)}>
  <SymbolLayer
    id="ownship-symbol"
    layout={{
      'icon-image': 'updraft-sdf:glider',
      'icon-rotation-alignment': 'map',
      'icon-rotate': ['get', 'track'],
      'icon-allow-overlap': true,
    }}
    paint={{
      'icon-color': '#5ee9b5', // --color-emerald-300
      'icon-halo-color': '#0f172b', // --color-slate-900
      'icon-halo-width': 1.5,
    }}
  />
</GeoJSONSource>

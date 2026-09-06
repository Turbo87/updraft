<script lang="ts">
  import type { LatLon } from '$lib/protocol/generated/LatLon';

  import { GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { COLOR_EMERALD_300, COLOR_SLATE_900 } from './colors.generated';
  import { ownshipFeature } from './ownship';

  type Props = { position: LatLon; trackDegrees: number | null };

  let { position, trackDegrees }: Props = $props();
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
      'icon-color': COLOR_EMERALD_300,
      'icon-halo-color': COLOR_SLATE_900,
      'icon-halo-width': 1.5,
    }}
  />
</GeoJSONSource>

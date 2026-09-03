<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { HillshadeLayer, RasterDEMTileSource } from 'svelte-maplibre-gl';

  const terrainUrl = convertFileSrc('terrain', 'updraft');
</script>

<RasterDEMTileSource
  id="terrain"
  tiles={[`${terrainUrl}/{z}/{x}/{y}.webp`]}
  url={`${terrainUrl}/metadata.json`}
>
  <HillshadeLayer
    id="terrain-hillshade"
    beforeId="waterway"
    paint={{
      'hillshade-method': 'igor',
      'hillshade-shadow-color': 'rgba(0, 0, 0, 0.65)',
    }}
  />
</RasterDEMTileSource>

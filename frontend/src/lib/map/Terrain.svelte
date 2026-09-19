<script lang="ts">
  import type { DerivedWindInstruments } from '$lib/protocol/generated/DerivedWindInstruments';
  import type { HillshadeDirection } from './terrain-style';

  import { convertFileSrc } from '@tauri-apps/api/core';
  import { ColorReliefLayer, HillshadeLayer, RasterDEMTileSource } from 'svelte-maplibre-gl';

  import { hillshadeLighting } from './terrain-style';

  type Props = {
    hillshadeDirection?: HillshadeDirection;
    wind?: DerivedWindInstruments | null;
  };

  let { hillshadeDirection = 'fixed', wind }: Props = $props();

  const terrainUrl = convertFileSrc('terrain/0', 'updraft');
  const lighting = $derived(hillshadeLighting(hillshadeDirection, wind));
</script>

<RasterDEMTileSource
  id="terrain"
  tiles={[`${terrainUrl}/{z}/{x}/{y}.webp`]}
  url={`${terrainUrl}/metadata.json`}
>
  <ColorReliefLayer
    id="terrain-color-relief"
    beforeId="water"
    paint={{
      'color-relief-opacity': 0.5,
      'color-relief-color': [
        'interpolate',
        ['linear'],
        ['elevation'],
        0,
        '#FFFFFF',
        30,
        '#FFFFFF',
        80,
        '#F5F5F0',
        350,
        '#F0F3E0',
        500,
        '#ededc7',
        1050,
        '#f2e0b1',
        1800,
        '#e0c8a8',
        2300,
        '#DAB3A8',
        3100,
        '#CFD7D1',
        3800,
        '#F5F5F5',
        6000,
        '#C6E7F7',
        9000,
        '#DCDCFF',
      ],
    }}
  />
  <HillshadeLayer
    id="terrain-hillshade"
    beforeId="waterway"
    paint={{
      ...lighting,
      'hillshade-method': 'igor',
      'hillshade-shadow-color': 'rgba(0, 0, 0, 0.65)',
    }}
  />
</RasterDEMTileSource>

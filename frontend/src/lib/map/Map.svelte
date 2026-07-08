<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import type { Map } from 'maplibre-gl';
  import { MapLibre } from 'svelte-maplibre-gl';
  import Ownship from './Ownship.svelte';
  import type { OwnshipPosition } from './ownship';

  // Fixed placeholder position (EDKA Aachen-Merzbrück) until core state drives
  // the map in the `map-position` step.
  const ownship: OwnshipPosition = { longitude: 6.186, latitude: 50.823, track: 45 };

  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);

  async function loadSprites() {
    await map?.addSprite('updraft-sdf', `${window.location.origin}/sprites/updraft-sdf`);
    spritesLoaded = true;
  }
</script>

<MapLibre
  inlineStyle="height: 100%; width: 100%"
  style="https://tiles.openfreemap.org/styles/positron"
  autoloadGlobalCss={false}
  bind:map
  onload={loadSprites}
  center={[ownship.longitude, ownship.latitude]}
  zoom={11}
>
  {#if spritesLoaded}
    <Ownship position={ownship} />
  {/if}
</MapLibre>

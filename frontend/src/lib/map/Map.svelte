<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import type { Map } from 'maplibre-gl';
  import { MapLibre } from 'svelte-maplibre-gl';
  import type { OwnshipPosition } from '$lib/protocol/generated/OwnshipPosition';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import Ownship from './Ownship.svelte';

  let { position }: { position: OwnshipPosition | null } = $props();

  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);
  let center = $derived<[number, number]>(
    position ? [position.location.longitude, position.location.latitude] : [6.186, 50.823],
  );

  async function loadSprites() {
    await map?.addSprite('updraft-sdf', `${window.location.origin}/sprites/updraft-sdf`);
    spritesLoaded = true;
  }
</script>

<div class="map-container">
  <MapLibre
    inlineStyle="height: 100%; width: 100%"
    style="https://tiles.openfreemap.org/styles/positron"
    autoloadGlobalCss={false}
    bind:map
    onload={loadSprites}
    {center}
    zoom={11}
  >
    {#if spritesLoaded && position}
      <Ownship {position} />
    {/if}
  </MapLibre>
  <MapDebugOverlay {map} />
</div>

<style>
  .map-container {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>

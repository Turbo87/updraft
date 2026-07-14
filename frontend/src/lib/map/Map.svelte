<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import type { Map } from 'maplibre-gl';
  import { MapLibre } from 'svelte-maplibre-gl';
  import type { PositionFix } from '$lib/protocol/generated/PositionFix';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import Ownship from './Ownship.svelte';
  import { positionCoordinates } from './ownship';

  const DEFAULT_CENTER: [number, number] = [6.186, 50.823];

  let { position }: { position: PositionFix | null } = $props();

  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);
  const center = $derived(position ? positionCoordinates(position) : DEFAULT_CENTER);

  function loadSprites() {
    if (!map) return;

    map.addSprite('updraft-sdf', `${window.location.origin}/sprites/updraft-sdf`);
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

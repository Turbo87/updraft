<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import type { Map, MapMouseEvent } from 'maplibre-gl';
  import { MapLibre } from 'svelte-maplibre-gl';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import Ownship from './Ownship.svelte';
  import type { OwnshipPosition } from './ownship';

  // Fixed placeholder position (EDKA Aachen-Merzbrück) until core state drives
  // the map in the `map-position` step.
  const ownship: OwnshipPosition = { longitude: 6.186, latitude: 50.823, track: 45 };

  // A tap encodes the hit point into the URL, opening the "what's here?" dialog.
  function query(event: MapMouseEvent) {
    goto(resolve(`/whats-here/@${event.lngLat.lat.toFixed(5)},${event.lngLat.lng.toFixed(5)}`));
  }

  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);

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
    onclick={query}
    center={[ownship.longitude, ownship.latitude]}
    zoom={11}
  >
    {#if spritesLoaded}
      <Ownship position={ownship} />
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

<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import type { Map, StyleSpecification } from 'maplibre-gl';
  import { MapLibre } from 'svelte-maplibre-gl';
  import type { OwnshipPosition } from '$lib/protocol/generated/OwnshipPosition';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import Ownship from './Ownship.svelte';

  let { position }: { position: OwnshipPosition | null } = $props();

  let testMode = new URLSearchParams(window.location.search).has('testMode');
  let minimalStyle = {
    version: 8,
    sources: {},
    layers: [
      {
        id: 'background',
        type: 'background',
        paint: { 'background-color': '#eef2f7' },
      },
    ],
  } satisfies StyleSpecification;
  let mapStyle = testMode ? minimalStyle : 'https://tiles.openfreemap.org/styles/positron';
  let mapOptions = { ...(testMode && { fadeDuration: 0 }) };
  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);
  let center = $derived<[number, number]>(
    position ? [position.location.longitude, position.location.latitude] : [6.186, 50.823],
  );

  async function loadSprites() {
    await map?.addSprite('updraft-sdf', `${window.location.origin}/sprites/updraft-sdf`);
    spritesLoaded = true;
  }

  $effect(() => {
    if (!testMode || !map) {
      return;
    }

    window.updraftTest = { map };
    return () => {
      delete window.updraftTest;
    };
  });
</script>

<div class="map-container">
  <MapLibre
    inlineStyle="height: 100%; width: 100%"
    style={mapStyle}
    {...mapOptions}
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

<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import type { Map, StyleSpecification } from 'maplibre-gl';
  import { MapLibre } from 'svelte-maplibre-gl';
  import type { OwnshipPosition } from '$lib/protocol/generated/OwnshipPosition';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import Ownship from './Ownship.svelte';

  let { position }: { position: OwnshipPosition | null } = $props();

  // Initial view only (EDKA Aachen-Merzbrück): the map never recenters on
  // position updates, so panning stays under the user's control. The
  // "return to position" affordance comes later (see docs/design/frontend.md).
  const INITIAL_CENTER: [number, number] = [6.186, 50.823];

  // Test mode (see docs/design/testing.md): offline inline style instead of
  // online tiles, no symbol fading, and the map exposed for Playwright.
  const testMode = new URLSearchParams(window.location.search).has('testMode');

  const TEST_STYLE: StyleSpecification = {
    version: 8,
    sources: {},
    layers: [{ id: 'background', type: 'background', paint: { 'background-color': '#e6e4e0' } }],
  };

  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);

  async function loadSprites() {
    await map?.addSprite('updraft-sdf', `${window.location.origin}/sprites/updraft-sdf`);
    spritesLoaded = true;
  }

  $effect(() => {
    if (testMode && map) {
      window.updraftTest = { map };
    }
  });
</script>

<div class="map-container">
  <MapLibre
    inlineStyle="height: 100%; width: 100%"
    style={testMode ? TEST_STYLE : 'https://tiles.openfreemap.org/styles/positron'}
    autoloadGlobalCss={false}
    fadeDuration={testMode ? 0 : undefined}
    bind:map
    onload={loadSprites}
    center={INITIAL_CENTER}
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

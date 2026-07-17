<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';

  import type { Map, StyleSpecification } from 'maplibre-gl';
  import type { PositionFix } from '$lib/protocol/generated/PositionFix';

  import { MapLibre } from 'svelte-maplibre-gl';

  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import { positionCoordinates } from './ownship';
  import Ownship from './Ownship.svelte';

  type TestWindow = Window & {
    __updraftTest?: { map: Map };
  };

  const DEFAULT_CENTER: [number, number] = [6.186, 50.823];
  const TEST_STYLE: StyleSpecification = {
    version: 8,
    sources: {},
    layers: [],
  };

  let { position, testMode = false }: { position: PositionFix | null; testMode?: boolean } =
    $props();

  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);
  const center = $derived(position ? positionCoordinates(position) : DEFAULT_CENTER);
  const mapStyle = $derived(
    testMode ? TEST_STYLE : 'https://tiles.openfreemap.org/styles/positron',
  );

  $effect(() => {
    if (!testMode || !map) return;

    let testWindow = window as TestWindow;
    testWindow.__updraftTest = { map };

    return () => {
      delete testWindow.__updraftTest;
    };
  });

  function loadSprites() {
    if (!map) return;

    map.addSprite('updraft-sdf', `${window.location.origin}/sprites/updraft-sdf`);
    spritesLoaded = true;
  }
</script>

<div class="map-container">
  <MapLibre
    inlineStyle="height: 100%; width: 100%"
    style={mapStyle}
    {...testMode ? { fadeDuration: 0 } : {}}
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

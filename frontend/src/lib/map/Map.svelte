<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'svelte-maplibre-gl/vite';

  import type { GeoJSONSourceSpecification, Map, StyleSpecification } from 'maplibre-gl';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { MapLibre } from 'svelte-maplibre-gl';

  import Airspace from './Airspace.svelte';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import { positionCoordinates } from './ownship';
  import Ownship from './Ownship.svelte';
  import Traffic from './Traffic.svelte';

  type TestWindow = Window & {
    __updraftTest?: { map: Map };
    __updraftTestAirspaceData?: GeoJSONSourceSpecification['data'];
  };

  const DEFAULT_CENTER: [number, number] = [6.186, 50.823];
  const TEST_STYLE: StyleSpecification = {
    version: 8,
    sources: {},
    layers: [],
  };

  let {
    airspace,
    instruments,
    traffic,
    units,
    testMode = false,
    testAirspaceData,
  }: {
    airspace: AirspaceStatus;
    instruments: Instruments;
    traffic: TrafficStore;
    units: UnitSettings;
    testMode?: boolean;
    testAirspaceData?: GeoJSONSourceSpecification['data'];
  } = $props();

  let map: Map | undefined = $state();
  let spritesLoaded = $state(false);
  const position = $derived(instruments.position);
  const center = $derived(position ? positionCoordinates(position) : DEFAULT_CENTER);
  const mapStyle = $derived(
    testMode ? TEST_STYLE : 'https://tiles.openfreemap.org/styles/positron',
  );
  const inlineAirspaceData = $derived(
    testMode ? (testAirspaceData ?? (window as TestWindow).__updraftTestAirspaceData) : undefined,
  );
  const airspaceData = $derived(
    airspace.type === 'active'
      ? (inlineAirspaceData ?? `updraft://localhost/airspace.geojson?v=${airspace.generation}`)
      : null,
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
    {#if spritesLoaded}
      <Traffic {traffic} altitudeUnit={units.altitude} />
      {#if position}
        <Ownship {position} trackDegrees={instruments.trackDegrees} />
      {/if}
      {#if airspaceData}
        <Airspace data={airspaceData} beforeId="traffic-fixed" />
      {/if}
    {/if}
  </MapLibre>
  <MapDebugOverlay {map} {instruments} {units} />
</div>

<style>
  .map-container {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>

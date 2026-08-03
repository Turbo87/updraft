<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'svelte-maplibre-gl/vite';

  import type { GeoJSONSourceSpecification, StyleSpecification } from 'maplibre-gl';
  import type { MapState } from '$lib/map-state.svelte';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { MapLibre } from 'svelte-maplibre-gl';

  import Airspace from './Airspace.svelte';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import { positionCoordinates } from './ownship';
  import Ownship from './Ownship.svelte';
  import ReturnToPositionButton from './ReturnToPositionButton.svelte';
  import Traffic from './Traffic.svelte';

  type TestWindow = Window & {
    __updraftTestAirspaceData?: GeoJSONSourceSpecification['data'];
  };

  const FOLLOW_DURATION_MS = 300;
  const TEST_STYLE: StyleSpecification = {
    version: 8,
    sources: {},
    layers: [],
  };

  let {
    airspace,
    instruments,
    mapState,
    traffic,
    units,
    testMode = false,
    testAirspaceData,
  }: {
    airspace: AirspaceStatus;
    instruments: Instruments;
    mapState: MapState;
    traffic: TrafficStore;
    units: UnitSettings;
    testMode?: boolean;
    testAirspaceData?: GeoJSONSourceSpecification['data'];
  } = $props();

  let spritesLoaded = $state(false);
  const map = $derived(mapState.map);
  const position = $derived(instruments.position);
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
    if (!map || !mapState.followMode || !position) return;

    map.easeTo({
      center: positionCoordinates(position),
      duration: testMode ? 0 : FOLLOW_DURATION_MS,
    });
  });

  function enterManualMode() {
    mapState.followMode = false;
  }

  function resumeFollowing() {
    map?.stop();
    mapState.followMode = true;
  }

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
    attributionControl={false}
    autoloadGlobalCss={false}
    bind:map={mapState.map}
    bind:bearing={mapState.bearing}
    bind:center={mapState.center}
    bind:pitch={mapState.pitch}
    bind:zoom={mapState.zoom}
    ondragstart={enterManualMode}
    onload={loadSprites}
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
  {#if !mapState.followMode}
    <ReturnToPositionButton onClick={resumeFollowing} />
  {/if}
  <MapDebugOverlay {map} {instruments} {units} />
</div>

<style>
  .map-container {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>

<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'svelte-maplibre-gl/vite';

  import type { GeoJSONSourceSpecification, MapMouseEvent } from 'maplibre-gl';
  import type { MapState } from '$lib/map-state.svelte';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { convertFileSrc } from '@tauri-apps/api/core';
  import { MapLibre } from 'svelte-maplibre-gl';

  import Airspace from './Airspace.svelte';
  import { BASEMAP_MIN_ZOOM, getBasemapStyle } from './basemap-style';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import { positionCoordinates } from './ownship';
  import Ownship from './Ownship.svelte';
  import ReturnToPositionButton from './ReturnToPositionButton.svelte';
  import Terrain from './Terrain.svelte';
  import Traffic from './Traffic.svelte';

  type TestWindow = Window & {
    __updraftTestAirspaceData?: GeoJSONSourceSpecification['data'];
  };

  const FOLLOW_DURATION_MS = 300;
  let {
    airspace,
    instruments,
    mapState,
    traffic,
    units,
    testMode = false,
    testAirspaceData,
    onInspect,
  }: {
    airspace: AirspaceStatus;
    instruments: Instruments;
    mapState: MapState;
    traffic: TrafficStore;
    units: UnitSettings;
    testMode?: boolean;
    testAirspaceData?: GeoJSONSourceSpecification['data'];
    onInspect?: (position: LatLon) => void;
  } = $props();

  let spritesLoaded = $state(false);
  let showTrafficHitAreas = $state(false);
  const map = $derived(mapState.map);
  const gps = $derived(instruments.gps);
  const position = $derived(gps?.position ?? null);
  const mapStyle = $derived(getBasemapStyle(testMode, window.location.origin));
  const inlineAirspaceData = $derived(
    testMode ? (testAirspaceData ?? (window as TestWindow).__updraftTestAirspaceData) : undefined,
  );
  const airspaceData = $derived(
    airspace.type === 'active'
      ? (inlineAirspaceData ??
          (testMode
            ? null
            : `${convertFileSrc('airspace.geojson', 'updraft')}?v=${airspace.generation}`))
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

  function inspectMapPosition(event: MapMouseEvent) {
    onInspect?.({
      latitudeDegrees: event.lngLat.lat,
      longitudeDegrees: event.lngLat.lng,
    });
  }
</script>

<div class="map-container">
  <MapLibre
    inlineStyle="height: 100%; width: 100%"
    style={mapStyle}
    minZoom={BASEMAP_MIN_ZOOM}
    {...testMode ? { fadeDuration: 0 } : {}}
    attributionControl={false}
    autoloadGlobalCss={false}
    bind:map={mapState.map}
    bind:bearing={mapState.bearing}
    bind:center={mapState.center}
    bind:pitch={mapState.pitch}
    bind:zoom={mapState.zoom}
    onclick={inspectMapPosition}
    ondragstart={enterManualMode}
    onload={loadSprites}
  >
    {#if !testMode}
      <Terrain />
    {/if}
    {#if spritesLoaded}
      <Traffic {traffic} altitudeUnit={units.altitude} {showTrafficHitAreas} />
      {#if position}
        <Ownship {position} trackDegrees={gps?.trackDegrees ?? null} />
      {/if}
      {#if airspaceData}
        <Airspace data={airspaceData} beforeId="traffic-fixed" />
      {/if}
    {/if}
  </MapLibre>
  {#if !mapState.followMode}
    <ReturnToPositionButton onClick={resumeFollowing} />
  {/if}
  <MapDebugOverlay {map} {instruments} {units} bind:showTrafficHitAreas />
</div>

<style>
  .map-container {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>

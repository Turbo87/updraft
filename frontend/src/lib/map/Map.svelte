<script lang="ts">
  import type { ClimbAverageMethod } from '#lib/protocol/generated/ClimbAverageMethod.js';
  import type { Navigation as NavigationState } from '#lib/protocol/generated/Navigation.js';
  import type { Task } from '#lib/protocol/generated/Task.js';

  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'svelte-maplibre-gl/vite';

  import type { GeoJSONSourceSpecification, MapEventType, MapMouseEvent } from 'maplibre-gl';
  import type { UpdraftClient } from '#lib/client/index.js';
  import type { MapState } from '#lib/map-state.svelte.js';
  import type { AirspaceStatus } from '#lib/protocol/generated/AirspaceStatus.js';
  import type { HillshadeDirection } from '#lib/protocol/generated/HillshadeDirection.js';
  import type { Instruments } from '#lib/protocol/generated/Instruments.js';
  import type { LatLon } from '#lib/protocol/generated/LatLon.js';
  import type { TrailColorMode } from '#lib/protocol/generated/TrailColorMode.js';
  import type { UnitSettings } from '#lib/protocol/generated/UnitSettings.js';
  import type { WaypointStatus } from '#lib/protocol/generated/WaypointStatus.js';
  import type { TrafficStore } from '#lib/stores/traffic.svelte.js';
  import type { TrailStore } from '#lib/stores/trail.svelte.js';
  import type { GroundMotion } from './dead-reckoning';

  import { untrack } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { MapLibre } from 'svelte-maplibre-gl';

  import Airspace from './Airspace.svelte';
  import Arrivals from './Arrivals.svelte';
  import { BASEMAP_MIN_ZOOM, getBasemapStyle } from './basemap-style';
  import { DEAD_RECKONING_LIMIT_SECONDS, deadReckon, groundMotion } from './dead-reckoning';
  import MapDebugOverlay from './MapDebugOverlay.svelte';
  import Navigation from './Navigation.svelte';
  import { positionCoordinates } from './ownship';
  import Ownship from './Ownship.svelte';
  import ReturnToPositionButton from './ReturnToPositionButton.svelte';
  import TaskRoute from './TaskRoute.svelte';
  import Terrain from './Terrain.svelte';
  import Traffic from './Traffic.svelte';
  import Trail from './Trail.svelte';
  import Waypoints from './Waypoints.svelte';

  type TestWindow = Window & {
    __updraftTestWaypointData?: GeoJSONSourceSpecification['data'];
    __updraftTestAirspaceData?: GeoJSONSourceSpecification['data'];
  };

  const RETURN_DURATION_MS = 300;
  type Props = {
    task?: Task;
    navigation?: NavigationState | null;
    climbAverageMethod?: ClimbAverageMethod;
    client?: UpdraftClient;
    airspace: AirspaceStatus;
    basemapGeneration?: number;
    terrainGeneration?: number;
    waypoints?: WaypointStatus;
    testWaypointData?: GeoJSONSourceSpecification['data'];
    instruments: Instruments;
    hillshadeDirection: HillshadeDirection;
    mapState: MapState;
    traffic: TrafficStore;
    trail: TrailStore;
    trailColorMode: TrailColorMode;
    units: UnitSettings;
    testMode?: boolean;
    testAirspaceData?: GeoJSONSourceSpecification['data'];
    onInspect?: (position: LatLon) => void;
  };

  let {
    task,
    navigation = null,
    climbAverageMethod = 'smoothed20s',
    client,
    airspace,
    basemapGeneration = 0,
    terrainGeneration = 0,
    instruments,
    hillshadeDirection,
    mapState,
    traffic,
    trail,
    trailColorMode,
    units,
    testMode = false,
    testAirspaceData,
    testWaypointData,
    waypoints = { generation: 0, sources: [] },
    onInspect,
  }: Props = $props();

  let spritesLoaded = $state(false);
  let showHitAreas = $state(false);
  let arrivalsReady = $state(false);
  const map = $derived(mapState.map);
  const gps = $derived(instruments.gps);
  const latitudeDegrees = $derived(gps?.position.latitudeDegrees);
  const longitudeDegrees = $derived(gps?.position.longitudeDegrees);
  const position = $derived(
    latitudeDegrees === undefined || longitudeDegrees === undefined
      ? null
      : { latitudeDegrees, longitudeDegrees },
  );
  const trackDegrees = $derived(gps?.trackDegrees ?? null);
  const groundSpeedMetersPerSecond = $derived(gps?.groundSpeedMetersPerSecond ?? null);
  const turnRateDegreesPerSecond = $derived(gps?.turnRateDegreesPerSecond ?? null);
  const stale = $derived(gps?.stale ?? true);
  const motion = $derived(
    groundMotion({ groundSpeedMetersPerSecond, trackDegrees, turnRateDegreesPerSecond, stale }),
  );

  type OwnshipSymbol = { position: LatLon; trackDegrees: number | null };
  type ReceivedOwnship = OwnshipSymbol & { motion: GroundMotion | null; receivedAt: number };
  /** The latest received ownship position and its frontend receive time. */
  let received: ReceivedOwnship | null = null;
  /** The dead-reckoned ownship that the map shows. */
  let displayed = $state.raw<OwnshipSymbol | null>(null);
  /** The start of the Return to position ease. */
  let returning: { center: LatLon; startedAt: number } | null = null;
  let frame: number | undefined;
  /** Pointers that are down on the map and can start a gesture. */
  let pressedPointers: number[] = [];
  const mapStyle = $derived(getBasemapStyle(testMode, window.location.origin, basemapGeneration));
  const inlineAirspaceData = $derived(
    testMode ? (testAirspaceData ?? (window as TestWindow).__updraftTestAirspaceData) : undefined,
  );
  const airspaceData = $derived(
    airspace.sources.some((source) => source.type === 'active')
      ? (inlineAirspaceData ??
          (testMode
            ? null
            : `${convertFileSrc('airspace.geojson', 'updraft')}?v=${airspace.generation}`))
      : null,
  );

  const waypointData = $derived(
    waypoints.sources.some((source) => source.type === 'active')
      ? testMode
        ? (testWaypointData ?? (window as TestWindow).__updraftTestWaypointData ?? null)
        : `${convertFileSrc('waypoints.geojson', 'updraft')}?v=${waypoints.generation}`
      : null,
  );

  $effect(() => {
    void waypointData;
    mapState.waypointSourceStatus = 'loading';
  });

  function handleSourceData(event: MapEventType['sourcedata']) {
    if (event.sourceId === 'waypoints' && event.sourceDataType === 'content') {
      mapState.waypointSourceStatus = 'ready';
    }
  }

  function setArrivalsReady(ready: boolean) {
    arrivalsReady = ready;
  }

  function handleSourceError(event: MapEventType['error']) {
    if ('sourceId' in event && event.sourceId === 'waypoints') {
      mapState.waypointSourceStatus = 'failed';
    }
    if ('sourceId' in event && event.sourceId === 'arrivals') {
      console.error('Failed to load arrival resource', event.error);
    }
  }

  // Dead reckoning starts at the receive time, because the frontend clock
  // and the GPS clock are not synchronized.
  $effect(() => {
    let update = position && { position, trackDegrees, motion };
    untrack(() => receive(update));
  });

  $effect(() => {
    if (!map) return;
    // Zoom keeps the ownship at the center while the map follows it.
    let options = mapState.followMode ? { around: 'center' as const } : undefined;
    // `enable()` keeps the options of an enabled scroll zoom.
    map.scrollZoom.disable();
    map.scrollZoom.enable(options);
    map.touchZoomRotate.enable(options);
    untrack(requestFrame);
  });

  $effect(() => {
    if (!map) return;
    let container = map.getCanvasContainer();
    function press(event: PointerEvent) {
      pressedPointers.push(event.pointerId);
    }
    function release(event: PointerEvent) {
      pressedPointers = pressedPointers.filter((pointer) => pointer !== event.pointerId);
      requestFrame();
    }
    container.addEventListener('pointerdown', press);
    window.addEventListener('pointerup', release);
    window.addEventListener('pointercancel', release);
    return () => {
      container.removeEventListener('pointerdown', press);
      window.removeEventListener('pointerup', release);
      window.removeEventListener('pointercancel', release);
      pressedPointers = [];
    };
  });

  $effect(() => () => {
    if (frame !== undefined) cancelAnimationFrame(frame);
  });

  /**
   * Starts dead reckoning from a new position.
   *
   * Other changes only update a symbol that does not move, so that the
   * symbol keeps its dead-reckoned position until the next position.
   */
  function receive(update: Omit<ReceivedOwnship, 'receivedAt'> | null) {
    if (!update) {
      received = displayed = null;
      return;
    }
    let { latitudeDegrees, longitudeDegrees } = update.position;
    let moved =
      received?.position.latitudeDegrees !== latitudeDegrees ||
      received.position.longitudeDegrees !== longitudeDegrees;
    if (!moved && received?.motion) return;

    received = { ...update, receivedAt: performance.now() };
    displayed = { position: update.position, trackDegrees: update.trackDegrees };
    requestFrame();
  }

  function requestFrame() {
    frame ??= requestAnimationFrame(renderFrame);
  }

  /** Advances the ownship and the following camera, and requests the next frame while either moves. */
  function renderFrame() {
    frame = undefined;
    if (!received) return;

    let now = performance.now();
    let elapsedSeconds = (now - received.receivedAt) / 1000;
    let next = received.motion
      ? deadReckon(received.position, received.motion, elapsedSeconds)
      : { position: received.position, trackDegrees: received.trackDegrees };
    if (
      next.position.latitudeDegrees !== displayed?.position.latitudeDegrees ||
      next.position.longitudeDegrees !== displayed.position.longitudeDegrees ||
      next.trackDegrees !== displayed.trackDegrees
    ) {
      displayed = { position: next.position, trackDegrees: next.trackDegrees };
    }

    let reckoning = received.motion !== null && elapsedSeconds < DEAD_RECKONING_LIMIT_SECONDS;
    if (follow(next.position, now) || reckoning) requestFrame();
  }

  /**
   * Centers the map on the ownship and returns whether the camera needs more frames.
   *
   * `jumpTo()` stops gestures, inertia, and zoom animations, and it resets a
   * gesture that has not started yet. The camera waits while the map moves or
   * a pointer is down.
   */
  function follow(ownship: LatLon, now: number): boolean {
    if (!map || !mapState.followMode) return false;
    if (map.isMoving() || pressedPointers.length > 0) return true;

    let center = ownship;
    let progress = returning ? (now - returning.startedAt) / RETURN_DURATION_MS : 1;
    if (returning && progress < 1) {
      let eased = 1 - (1 - progress) ** 3;
      let { latitudeDegrees, longitudeDegrees } = returning.center;
      center = {
        latitudeDegrees: latitudeDegrees + (ownship.latitudeDegrees - latitudeDegrees) * eased,
        longitudeDegrees: longitudeDegrees + (ownship.longitudeDegrees - longitudeDegrees) * eased,
      };
    } else {
      returning = null;
    }

    let current = map.getCenter();
    if (current.lat !== center.latitudeDegrees || current.lng !== center.longitudeDegrees) {
      map.jumpTo({ center: positionCoordinates(center) });
    }
    return returning !== null;
  }

  function enterManualMode() {
    mapState.followMode = false;
  }

  function handleDragStart(event: MapEventType['dragstart']) {
    let { originalEvent } = event;
    if (!originalEvent) return;
    // A pinch zoom also pans with the center of its two touches. Desktop
    // WebKit and Firefox have no `TouchEvent` global to check for it.
    let pinch = 'touches' in originalEvent && originalEvent.touches.length > 1;
    if (!pinch) enterManualMode();
  }

  function resumeFollowing() {
    map?.stop();
    let center = map?.getCenter();
    returning =
      center && received
        ? {
            center: { latitudeDegrees: center.lat, longitudeDegrees: center.lng },
            startedAt: performance.now(),
          }
        : null;
    mapState.followMode = true;
    requestFrame();
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
    onsourcedata={handleSourceData}
    onerror={handleSourceError}
    onclick={inspectMapPosition}
    ondragstart={handleDragStart}
    onmoveend={requestFrame}
    onload={() => {
      spritesLoaded = true;
    }}
  >
    {#if !testMode}
      <!-- A URL change does not cancel pending DEM tile requests. -->
      {#key terrainGeneration}
        <Terrain
          generation={terrainGeneration}
          {hillshadeDirection}
          wind={instruments.derived?.wind ?? null}
          solarPosition={instruments.solarPosition}
        />
      {/key}
    {/if}
    {#if spritesLoaded}
      <Trail {trail} colorMode={trailColorMode} />
      <Traffic
        {climbAverageMethod}
        {traffic}
        altitudeUnit={units.altitude}
        verticalSpeedUnit={units.verticalSpeed}
        {showHitAreas}
      />
      {#if displayed}
        <Ownship position={displayed.position} trackDegrees={displayed.trackDegrees} />
      {/if}
      {#if airspaceData}
        <Airspace data={airspaceData} beforeId="trail-halo" />
      {/if}
      {#if waypointData}
        <Waypoints data={waypointData} {showHitAreas} showLandables={!arrivalsReady} />
        {#if client && map}
          <Arrivals
            {client}
            {map}
            generation={waypoints.generation}
            altitudeUnit={units.altitude}
            onReady={setArrivalsReady}
          />
        {/if}
      {/if}
    {/if}
    {#if task}<TaskRoute {task} />{/if}
    {#if navigation?.position}<Navigation position={navigation.position} ownship={position} />{/if}
  </MapLibre>
  {#if !mapState.followMode}
    <ReturnToPositionButton onClick={resumeFollowing} />
  {/if}
  <MapDebugOverlay {map} {instruments} {units} bind:showHitAreas />
</div>

<style>
  .map-container {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>

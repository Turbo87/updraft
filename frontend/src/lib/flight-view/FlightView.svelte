<script lang="ts">
  import type { UpdraftClient } from '$lib/client';
  import type { MapState } from '$lib/map-state.svelte';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
  import type { ClimbAverageMethod } from '$lib/protocol/generated/ClimbAverageMethod';
  import type { HillshadeDirection } from '$lib/protocol/generated/HillshadeDirection';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { Navigation as NavigationState } from '$lib/protocol/generated/Navigation';
  import type { PinnedTarget } from '$lib/protocol/generated/PinnedTarget';
  import type { Task } from '$lib/protocol/generated/Task';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { WaypointStatus } from '$lib/protocol/generated/WaypointStatus';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import Map from '$lib/map/Map.svelte';
  import MapOverlayControl from '$lib/MapOverlayControl.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { flightInfoboxes } from './infobox/fields';
  import InfoboxDock from './infobox/InfoboxDock.svelte';
  import PinnedTargets from './PinnedTargets.svelte';
  import TargetBar from './TargetBar.svelte';

  type Props = {
    pins?: PinnedTarget[];
    task?: Task;
    navigation?: NavigationState | null;
    climbAverageMethod?: ClimbAverageMethod;
    client?: UpdraftClient;
    airspace: AirspaceStatus;
    basemapGeneration?: number;
    terrainGeneration?: number;
    waypoints?: WaypointStatus;
    instruments: Instruments;
    hillshadeDirection: HillshadeDirection;
    mapState: MapState;
    traffic: TrafficStore;
    units: UnitSettings;
    testMode?: boolean;
  };

  let {
    task,
    navigation = null,
    pins = [],
    climbAverageMethod = 'smoothed20s',
    client,
    airspace,
    basemapGeneration,
    terrainGeneration,
    waypoints,
    instruments,
    hillshadeDirection,
    mapState,
    traffic,
    units,
    testMode = false,
  }: Props = $props();

  const infoboxes = $derived(flightInfoboxes(instruments, units));

  function openNearbyRoute(position: LatLon) {
    let path = resolve('/nearby/[latitude]/[longitude]', {
      latitude: position.latitudeDegrees.toFixed(6),
      longitude: position.longitudeDegrees.toFixed(6),
    });
    void goto(path);
  }
</script>

<section class="flight-view" aria-label={m.flight_view()}>
  <div class="main">
    {#if navigation}<TargetBar {navigation} {units} />{/if}
    <PinnedTargets {pins} {units} hasPrimary={navigation !== null} />
    <div class="map" class:has-target={navigation !== null || pins.some((pin) => !pin.primary)}>
      <Map
        {task}
        {navigation}
        {climbAverageMethod}
        {client}
        {airspace}
        {basemapGeneration}
        {terrainGeneration}
        {waypoints}
        {instruments}
        {hillshadeDirection}
        {mapState}
        {traffic}
        {units}
        {testMode}
        onInspect={openNearbyRoute}
      />
      <div class="overlay">
        <MapOverlayControl href="/settings" icon="i-mdi-menu" label={m.settings_heading()} />
      </div>
    </div>
  </div>
  <InfoboxDock {infoboxes} />
</section>

<style>
  .flight-view {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
  }

  .main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .map.has-target {
    --safe-area-top: 0px;
  }
  .map {
    --safe-area-bottom: 0px;
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  @media (orientation: landscape) {
    .flight-view {
      flex-direction: row;
    }
    .map {
      --safe-area-right: 0px;
      --safe-area-bottom: inherit;
    }
  }

  .overlay {
    position: absolute;
    top: calc(1rem + var(--safe-area-top));
    right: calc(1rem + var(--safe-area-right));
  }
</style>

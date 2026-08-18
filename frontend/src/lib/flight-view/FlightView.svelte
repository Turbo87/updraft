<script lang="ts">
  import type { MapState } from '$lib/map-state.svelte';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import Map from '$lib/map/Map.svelte';
  import MapOverlayControl from '$lib/MapOverlayControl.svelte';
  import { m } from '$lib/paraglide/messages.js';

  let {
    airspace,
    instruments,
    mapState,
    traffic,
    units,
    testMode = false,
  }: {
    airspace: AirspaceStatus;
    instruments: Instruments;
    mapState: MapState;
    traffic: TrafficStore;
    units: UnitSettings;
    testMode?: boolean;
  } = $props();

  function openNearbyRoute(position: LatLon) {
    let path = resolve('/nearby/[latitude]/[longitude]', {
      latitude: position.latitudeDegrees.toFixed(6),
      longitude: position.longitudeDegrees.toFixed(6),
    });
    void goto(path);
  }
</script>

<section class="flight-view" aria-label={m.flight_view()}>
  <Map
    {airspace}
    {instruments}
    {mapState}
    {traffic}
    {units}
    {testMode}
    onInspect={openNearbyRoute}
  />
  <div class="overlay">
    <MapOverlayControl href="/settings" icon="i-mdi-menu" label={m.settings_heading()} />
  </div>
</section>

<style>
  .flight-view {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .overlay {
    position: absolute;
    top: calc(1rem + var(--safe-area-top));
    right: calc(1rem + var(--safe-area-right));
  }
</style>

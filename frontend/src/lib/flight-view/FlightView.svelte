<script lang="ts">
  import type { MapState } from '$lib/map-state.svelte';
  import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { resolve } from '$app/paths';

  import Map from '$lib/map/Map.svelte';
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
</script>

<section class="flight-view" aria-label={m.flight_view()}>
  <Map {airspace} {instruments} {mapState} {traffic} {units} {testMode} />
  <div class="overlay">
    <a href={resolve('/settings')}>{m.settings_heading()}</a>
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
    top: 0.5rem;
    right: 0.5rem;
  }

  a {
    display: block;
    padding: 0.5rem 0.75rem;
    border-radius: 0.5rem;
    background-color: var(--color-overlay-surface);
    color: var(--color-overlay-text);
  }
</style>

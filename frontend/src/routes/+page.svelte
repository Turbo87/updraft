<script lang="ts">
  import Map from '$lib/map/Map.svelte';
  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import { ApplicationState, connectApplicationState } from '$lib/protocol/state.svelte';

  const appState = new ApplicationState();
  $effect(() => connectApplicationState(appState));
</script>

<div class="map">
  <Map position={appState.position} positionStale={appState.positionStale} />
  <div class="overlay">
    <LocaleSwitcher />
  </div>
  {#if appState.trackDistance > 0}
    <p class="track-distance">{(appState.trackDistance / 1000).toFixed(1)} km</p>
  {/if}
</div>

<style>
  .map {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .overlay {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
  }

  .track-distance {
    position: absolute;
    bottom: 0.5rem;
    right: 0.5rem;
    margin: 0;
    padding: 0.25rem 0.5rem;
    background: rgb(255 255 255 / 85%);
    border-radius: 0.25rem;
    font-variant-numeric: tabular-nums;
  }
</style>

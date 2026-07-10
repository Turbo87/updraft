<script lang="ts">
  import Map from '$lib/map/Map.svelte';
  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import { ApplicationState, connectApplicationState } from '$lib/protocol/state.svelte';

  const appState = new ApplicationState();
  $effect(() => connectApplicationState(appState));
</script>

<div class="map">
  <Map position={appState.position} />
  <div class="overlay">
    <LocaleSwitcher />
  </div>
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
</style>

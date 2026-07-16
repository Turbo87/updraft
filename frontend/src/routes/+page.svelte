<script lang="ts">
  import { browser } from '$app/environment';

  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import Map from '$lib/map/Map.svelte';
  import { HttpUpdraftClient } from '$lib/protocol/client';
  import { ApplicationState } from '$lib/protocol/state.svelte';

  const state = new ApplicationState();
  const client = new HttpUpdraftClient();
  const testMode = browser && new URLSearchParams(window.location.search).get('testMode') === '1';

  $effect(() => client.subscribe(state));
</script>

<div class="map">
  <Map position={state.flight.position} {testMode} />
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

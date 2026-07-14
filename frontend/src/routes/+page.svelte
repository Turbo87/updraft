<script lang="ts">
  import Map from '$lib/map/Map.svelte';
  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import { HttpUpdraftClient } from '$lib/protocol/client';
  import { ApplicationState } from '$lib/protocol/state.svelte';

  const state = new ApplicationState();
  const client = new HttpUpdraftClient();

  $effect(() => client.subscribe(state));
</script>

<div class="map">
  <Map position={state.flight.position} />
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

<script lang="ts">
  import Map from '$lib/map/Map.svelte';
  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import { StateClient } from '$lib/protocol/state-client';
  import { ApplicationState } from '$lib/protocol/state.svelte';

  let state = new ApplicationState();
  let stateClient = new StateClient();

  $effect(() =>
    stateClient.subscribe({
      onSnapshot(snapshot) {
        state.applySnapshot(snapshot);
      },
      onChanges(changes) {
        state.applyChanges(changes);
      },
    }),
  );
</script>

<div class="map">
  <Map position={state.position} />
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

<script lang="ts">
  import type { GnssState } from '$lib/protocol/generated/GnssState';

  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import Map from '$lib/map/Map.svelte';
  import { m } from '$lib/paraglide/messages.js';

  let { gnss, testMode = false }: { gnss: GnssState | null; testMode?: boolean } = $props();
</script>

<section class="flight-view" aria-label={m.flight_view()}>
  <Map {gnss} {testMode} />
  <div class="overlay">
    <LocaleSwitcher />
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
</style>

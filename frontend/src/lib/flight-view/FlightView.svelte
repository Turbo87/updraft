<script lang="ts">
  import type { Instruments } from '$lib/protocol/generated/Instruments';

  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import Map from '$lib/map/Map.svelte';
  import { m } from '$lib/paraglide/messages.js';

  let { instruments, testMode = false }: { instruments: Instruments; testMode?: boolean } =
    $props();
</script>

<section class="flight-view" aria-label={m.flight_view()}>
  <Map {instruments} {testMode} />
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

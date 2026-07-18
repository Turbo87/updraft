<script lang="ts">
  import type { PositionFix } from '$lib/protocol/generated/PositionFix';

  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import Map from '$lib/map/Map.svelte';
  import { m } from '$lib/paraglide/messages.js';

  let { position, testMode = false }: { position: PositionFix | null; testMode?: boolean } =
    $props();
</script>

<section class="flight-view" aria-label={m.flight_view()}>
  <Map {position} {testMode} />
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

<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import type { GnssData } from '$lib/gnss';

  import { page } from '$app/state';

  import favicon from '$lib/assets/favicon.svg';
  import FlightView from '$lib/flight-view/FlightView.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';

  let { children } = $props();

  const PLACEHOLDER: GnssData = {
    position: { status: 'unavailable' },
    altitudeMeters: { status: 'unavailable' },
    trackDegrees: { status: 'unavailable' },
    groundSpeedMetersPerSecond: { status: 'unavailable' },
  };
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<div class="app">
  <FlightView gnss={PLACEHOLDER} {testMode} />
  {#if page.url.pathname !== '/'}
    <div class="route-content">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .app,
  .route-content {
    position: absolute;
    inset: 0;
  }
</style>

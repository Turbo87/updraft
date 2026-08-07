<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import TrafficDetails from './TrafficDetails.svelte';

  const { instruments, settings, traffic } = getAppContext();
  const trafficId = $derived(page.params.id);
  const locale = $derived(settings.current.locale ?? getLocale());

  function goBack() {
    history.back();
  }
</script>

<main>
  <nav>
    <button type="button" onclick={goBack}>{m.traffic_back()}</button>
    <a href={resolve('/')}>{m.traffic_map()}</a>
  </nav>

  {#if !traffic.initialized}
    <p>{m.traffic_details_loading()}</p>
  {:else if !trafficId}
    <p>{m.traffic_not_found()}</p>
  {:else}
    <TrafficDetails
      altitudeUnit={settings.current.units.altitude}
      distanceUnit={settings.current.units.distance}
      id={trafficId}
      {instruments}
      {locale}
      {traffic}
    />
  {/if}
</main>

<style>
  main {
    box-sizing: border-box;
    min-height: 100%;
    padding: 1.5rem;
    overflow: auto;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  nav {
    display: flex;
    gap: 1rem;
    align-items: center;
    margin-block-end: 1rem;
  }
</style>

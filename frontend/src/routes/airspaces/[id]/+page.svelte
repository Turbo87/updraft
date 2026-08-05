<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import AirspaceDetails from './AirspaceDetails.svelte';

  const { airspace, mapState, settings } = getAppContext();
  const airspaceId = $derived(parseAirspaceId(page.params.id));
  const locale = $derived(settings.current.locale ?? getLocale());

  function parseAirspaceId(value: string | undefined): number | null {
    let id = Number(value);
    return Number.isSafeInteger(id) && id >= 0 ? id : null;
  }

  function goBack() {
    history.back();
  }
</script>

<main>
  <nav>
    <button type="button" onclick={goBack}>{m.airspace_back()}</button>
    <a href={resolve('/')}>{m.airspace_map()}</a>
  </nav>

  {#if airspaceId === null}
    <p>{m.airspace_not_found()}</p>
  {:else if !airspace.initialized}
    <p>{m.airspace_details_loading()}</p>
  {:else if airspace.current.type !== 'active'}
    <p>{m.airspace_not_found()}</p>
  {:else if mapState.map}
    <AirspaceDetails
      altitudeUnit={settings.current.units.altitude}
      id={airspaceId}
      {locale}
      map={mapState.map}
    />
  {:else}
    <p>{m.airspace_details_loading()}</p>
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

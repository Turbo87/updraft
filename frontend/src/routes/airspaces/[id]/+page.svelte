<script lang="ts">
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
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

{#if airspaceId !== null && airspace.initialized && airspace.current.type === 'active' && mapState.map}
  <AirspaceDetails
    altitudeUnit={settings.current.units.altitude}
    backLabel={m.airspace_back()}
    id={airspaceId}
    {locale}
    map={mapState.map}
    onBack={goBack}
  />
{:else}
  <ScreenScaffold backLabel={m.airspace_back()} onBack={goBack} title={m.airspace_label()}>
    <p class="empty-state">
      {airspaceId === null || (airspace.initialized && airspace.current.type !== 'active')
        ? m.airspace_not_found()
        : m.airspace_details_loading()}
    </p>
  </ScreenScaffold>
{/if}

<style>
  .empty-state {
    margin: 0;
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    color: var(--color-text-muted);
    font: var(--text-body);
  }
</style>

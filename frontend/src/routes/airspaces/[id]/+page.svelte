<script lang="ts">
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import AirspaceDetails from './AirspaceDetails.svelte';

  const { airspace, mapState, settings } = getAppContext();
  const airspaceId = $derived(
    page.params.id && /^\d+:\d+:\d+$/.test(page.params.id) ? page.params.id : null,
  );
  const currentId = $derived(
    airspaceId !== null &&
      airspaceId.startsWith(`${airspace.current.generation}:`) &&
      airspace.current.sources.some((source) => source.type === 'active'),
  );
  const locale = $derived(settings.current.locale ?? getLocale());

  function goBack() {
    history.back();
  }
</script>

{#if airspaceId !== null && airspace.initialized && currentId && mapState.map}
  {#key airspaceId}
    <AirspaceDetails
      altitudeUnit={settings.current.units.altitude}
      backLabel={m.airspace_back()}
      id={airspaceId}
      {locale}
      map={mapState.map}
      onBack={goBack}
    />
  {/key}
{:else}
  <ScreenScaffold backLabel={m.airspace_back()} onBack={goBack} title={m.airspace_label()}>
    <p class="empty-state">
      {airspaceId === null || (airspace.initialized && !currentId)
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

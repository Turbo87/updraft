<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { calculateDistanceAndBearing } from '$lib/geographic-position';
  import NearbyResultsScreen from '$lib/NearbyResultsScreen.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { convertDistance } from '$lib/units';
  import NearbyAirspaces from './NearbyAirspaces.svelte';
  import NearbyTraffic from './NearbyTraffic.svelte';
  import { parseNearbyRouteCoordinates } from './params';

  const { airspace, instruments, mapState, settings, traffic } = getAppContext();
  const selectedPosition = $derived(
    parseNearbyRouteCoordinates(page.params.latitude, page.params.longitude),
  );
  const locale = $derived(settings.current.locale ?? getLocale());
  const ownshipRelation = $derived(
    selectedPosition && instruments.current.gps?.position
      ? calculateDistanceAndBearing(instruments.current.gps.position, selectedPosition)
      : null,
  );
  const displayedOwnshipRelation = $derived.by(() => {
    if (!ownshipRelation) return null;

    let distanceUnit = settings.current.units.distance;
    return {
      distance: {
        value: convertDistance(ownshipRelation.distanceMeters, distanceUnit).toFixed(1),
        unit: distanceUnit,
      },
      bearing: { value: ownshipRelation.bearingDegrees.toFixed(0), unit: '°' },
    };
  });
  const summary = {
    arrivalHeight: { value: '—', stale: true },
    requiredGlideRatio: { value: '—', stale: true },
    terrainElevation: { value: '—', stale: true },
  };
</script>

{#if selectedPosition}
  {#snippet airspaces()}
    {#if airspace.initialized && mapState.map}
      {#key `${selectedPosition.latitudeDegrees}/${selectedPosition.longitudeDegrees}`}
        <NearbyAirspaces {airspace} {locale} map={mapState.map} position={selectedPosition} />
      {/key}
    {:else}
      <p>{m.loading_nearby_airspaces()}</p>
    {/if}
  {/snippet}

  {#snippet trafficResults()}
    {#if traffic.initialized && mapState.map}
      {#key `${selectedPosition.latitudeDegrees}/${selectedPosition.longitudeDegrees}`}
        <NearbyTraffic
          {locale}
          map={mapState.map}
          ownship={instruments.current.gps}
          position={selectedPosition}
          {traffic}
          units={settings.current.units}
        />
      {/key}
    {:else}
      <p>{m.loading_nearby_traffic()}</p>
    {/if}
  {/snippet}

  <NearbyResultsScreen
    {airspaces}
    backLabel={m.back_to_map()}
    ownshipRelation={displayedOwnshipRelation}
    position={selectedPosition}
    {summary}
    title={m.nearby_heading()}
    traffic={trafficResults}
  />
{:else}
  <main>
    <a class="back-link" href={resolve('/')}>{m.back_to_map()}</a>
    <p role="alert">{m.invalid_inspection()}</p>
  </main>
{/if}

<style>
  main {
    box-sizing: border-box;
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  .back-link {
    display: inline-block;
    margin-block-end: 1rem;
  }
</style>

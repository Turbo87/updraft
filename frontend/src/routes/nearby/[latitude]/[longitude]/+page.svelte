<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import { calculateDistanceAndBearing } from '$lib/geographic-position';
  import NearbyResultsScreen from '$lib/NearbyResultsScreen.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import PinTargetButton from '$lib/PinTargetButton.svelte';
  import ResponsiveCard from '$lib/ResponsiveCard.svelte';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import { convertDistance } from '$lib/units';
  import NearbyAirspaces from './NearbyAirspaces.svelte';
  import NearbyTraffic from './NearbyTraffic.svelte';
  import NearbyWaypoints from './NearbyWaypoints.svelte';
  import { parseNearbyRouteCoordinates } from './params';

  const { client, airspace, instruments, mapState, settings, traffic, waypoints } = getAppContext();
  const selectedPosition = $derived(
    parseNearbyRouteCoordinates(page.params.latitude, page.params.longitude),
  );
  let error = $state(false);
  let busy = $state(false);
  async function navigate() {
    if (!selectedPosition) return;
    busy = true;
    error = false;
    try {
      if (await client.setNavigationTarget({ type: 'mapPosition', ...selectedPosition })) {
        await goto(resolve('/'));
      } else {
        error = true;
      }
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
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
        {#key airspace.current.generation}
          <NearbyAirspaces {airspace} {locale} map={mapState.map} position={selectedPosition} />
        {/key}
      {/key}
    {:else}
      <ResponsiveCard>
        <p class="empty-results">{m.loading_nearby_airspaces()}</p>
      </ResponsiveCard>
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
      <ResponsiveCard>
        <p class="empty-results">{m.loading_nearby_traffic()}</p>
      </ResponsiveCard>
    {/if}
  {/snippet}

  {#snippet waypointResults()}
    {#if !waypoints.initialized || !mapState.map}
      <ResponsiveCard>
        <p class="empty-results">{m.waypoint_loading()}</p>
      </ResponsiveCard>
    {:else if !waypoints.current.sources.some((source) => source.type === 'active')}
      <ResponsiveCard>
        <p class="empty-results">{m.waypoint_none_nearby()}</p>
      </ResponsiveCard>
    {:else}
      {#key `${waypoints.current.generation}/${selectedPosition.latitudeDegrees}/${selectedPosition.longitudeDegrees}`}
        <NearbyWaypoints
          altitudeUnit={settings.current.units.altitude}
          map={mapState.map}
          position={selectedPosition}
          sourceStatus={mapState.waypointSourceStatus}
        />
      {/key}
    {/if}
  {/snippet}

  {#snippet actions()}<Button loading={busy} onclick={navigate}>{m.navigation_here()}</Button>
    {#if selectedPosition}<PinTargetButton
        target={{ type: 'mapPosition', ...selectedPosition }}
      />{/if}
  {/snippet}
  <NearbyResultsScreen
    {actions}
    {error}
    waypoints={waypointResults}
    {airspaces}
    backLabel={m.back_to_map()}
    ownshipRelation={displayedOwnshipRelation}
    position={selectedPosition}
    {summary}
    title={m.nearby_heading()}
    traffic={trafficResults}
  />
{:else}
  <ScreenScaffold backHref="/" backLabel={m.back_to_map()} title={m.nearby_heading()}>
    <p role="alert">{m.invalid_inspection()}</p>
  </ScreenScaffold>
{/if}

<style>
  .empty-results {
    margin: 0;
    padding-block: var(--space-5);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
    color: var(--color-text-muted);
    font: var(--text-body);
  }
</style>

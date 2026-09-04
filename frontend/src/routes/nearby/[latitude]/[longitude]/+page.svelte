<script lang="ts">
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { calculateDistanceAndBearing } from '$lib/geographic-position';
  import NearbyResultsScreen from '$lib/NearbyResultsScreen.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import { convertDistance } from '$lib/units';
  import NearbyAirspaces from './NearbyAirspaces.svelte';
  import NearbyTraffic from './NearbyTraffic.svelte';
  import NearbyWaypoints from './NearbyWaypoints.svelte';
  import { parseNearbyRouteCoordinates } from './params';

  const { airspace, instruments, mapState, settings, traffic, waypoints } = getAppContext();
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

  {#snippet waypointResults()}
    {#if !waypoints.initialized || !mapState.map}
      <p>{m.waypoint_loading()}</p>
    {:else if !waypoints.current.sources.some((source) => source.type === 'active')}
      <p>{m.waypoint_none_nearby()}</p>
    {:else}
      {#key `${waypoints.current.generation}/${selectedPosition.latitudeDegrees}/${selectedPosition.longitudeDegrees}`}
        <NearbyWaypoints
          map={mapState.map}
          position={selectedPosition}
          sourceStatus={mapState.waypointSourceStatus}
        />
      {/key}
    {/if}
  {/snippet}

  <NearbyResultsScreen
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

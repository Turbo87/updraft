<script lang="ts">
  import type { WaypointFeature } from '$lib/waypoints';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import WaypointLookup from './WaypointLookup.svelte';

  const { client, mapState, settings, waypoints } = getAppContext();
  let error = $state(false);
  async function navigate(waypoint: WaypointFeature) {
    error = false;
    try {
      let saved = await client.setNavigationTarget({
        type: 'waypoint',
        name: waypoint.properties.name,
        latitudeDegrees: waypoint.geometry.coordinates[1],
        longitudeDegrees: waypoint.geometry.coordinates[0],
        elevationMeters: waypoint.properties.elevationMeters,
      });
      if (!saved) {
        error = true;
        return;
      }
      await goto(resolve('/'));
    } catch {
      error = true;
    }
  }
  function goBack() {
    history.back();
  }
</script>

{#if waypoints.initialized && mapState.map && waypoints.current.sources.some((source) => source.type === 'active')}
  <WaypointLookup
    {error}
    map={mapState.map}
    sourceStatus={mapState.waypointSourceStatus}
    id={page.params.id ?? ''}
    generation={waypoints.current.generation}
    altitudeUnit={settings.current.units.altitude}
    onNavigate={navigate}
    onBack={goBack}
  />
{:else}
  <ScreenScaffold title={m.waypoints_heading()} backLabel={m.waypoint_back()} onBack={goBack}>
    <p>{waypoints.initialized ? m.waypoint_not_found() : m.waypoint_loading()}</p>
  </ScreenScaffold>
{/if}

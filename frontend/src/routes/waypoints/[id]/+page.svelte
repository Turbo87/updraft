<script lang="ts">
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import WaypointLookup from './WaypointLookup.svelte';

  const { mapState, settings, waypoints } = getAppContext();
  function goBack() {
    history.back();
  }
</script>

{#if waypoints.initialized && mapState.map && waypoints.current.sources.some((source) => source.type === 'active')}
  <WaypointLookup
    map={mapState.map}
    sourceStatus={mapState.waypointSourceStatus}
    id={page.params.id ?? ''}
    generation={waypoints.current.generation}
    altitudeUnit={settings.current.units.altitude}
    onBack={goBack}
  />
{:else}
  <ScreenScaffold title={m.waypoints_heading()} backLabel={m.waypoint_back()} onBack={goBack}>
    <p>{waypoints.initialized ? m.waypoint_not_found() : m.waypoint_loading()}</p>
  </ScreenScaffold>
{/if}

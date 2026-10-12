<script lang="ts">
  import { resolve } from '$app/paths';

  import { getAppContext } from '#lib/app-context.js';
  import MapSettings from '#lib/MapSettings.svelte';
  import { m } from '#lib/paraglide/messages.js';
  import ScreenScaffold from '#lib/ScreenScaffold.svelte';

  const { client, settings } = getAppContext();
</script>

<ScreenScaffold
  backHref={resolve('/settings')}
  backLabel={m.back_to_settings()}
  title={m.map_settings_heading()}
>
  <MapSettings
    direction={settings.current.hillshadeDirection}
    setDirection={(direction) => client.changeSetting({ type: 'hillshadeDirection', direction })}
    trailColorMode={settings.current.trailColorMode}
    setTrailColorMode={(mode) => client.changeSetting({ type: 'trailColorMode', mode })}
    trailLength={settings.current.trailLength}
    setTrailLength={(length) => client.changeSetting({ type: 'trailLength', length })}
    trailDriftCompensation={settings.current.trailDriftCompensation}
    setTrailDriftCompensation={(enabled) =>
      client.changeSetting({ type: 'trailDriftCompensation', enabled })}
    ownshipDeadReckoning={settings.current.ownshipDeadReckoning}
    setOwnshipDeadReckoning={(enabled) =>
      client.changeSetting({ type: 'ownshipDeadReckoning', enabled })}
  />
</ScreenScaffold>

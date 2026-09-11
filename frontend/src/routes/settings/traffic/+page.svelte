<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import TrafficSettings from '$lib/TrafficSettings.svelte';

  const { client, settings } = getAppContext();
</script>

<ScreenScaffold
  backHref="/settings"
  backLabel={m.back_to_settings()}
  title={m.traffic_settings_heading()}
>
  <TrafficSettings
    enabled={settings.current.flarmPositionCorrection}
    setEnabled={(enabled) => client.setFlarmPositionCorrection(enabled)}
  />
</ScreenScaffold>

<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import MapSettings from '$lib/MapSettings.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { client, settings } = getAppContext();
</script>

<ScreenScaffold
  backHref="/settings"
  backLabel={m.back_to_settings()}
  title={m.map_settings_heading()}
>
  <MapSettings
    direction={settings.current.hillshadeDirection}
    setDirection={(direction) => client.setHillshadeDirection(direction)}
  />
</ScreenScaffold>

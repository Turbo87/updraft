<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import VarioSettings from '$lib/VarioSettings.svelte';

  const { client, settings } = getAppContext();
</script>

<ScreenScaffold
  backHref="/settings"
  backLabel={m.back_to_settings()}
  title={m.vario_settings_heading()}
>
  <VarioSettings
    method={settings.current.climbAverageMethod}
    setMethod={(method) => client.setClimbAverageMethod(method)}
  />
</ScreenScaffold>

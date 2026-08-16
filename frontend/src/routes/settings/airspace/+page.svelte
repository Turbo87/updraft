<script lang="ts">
  import type { ImportAirspaceResult } from '$lib/client';

  import AirspaceSetting from '$lib/AirspaceSetting.svelte';
  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { client, airspace } = getAppContext();

  function importAirspace(): Promise<ImportAirspaceResult> {
    return client.importAirspace();
  }

  function removeAirspace(): Promise<void> {
    return client.removeAirspace();
  }
</script>

<ScreenScaffold backHref="/settings" backLabel={m.back_to_settings()} title={m.airspace_label()}>
  <AirspaceSetting status={airspace.current} onImport={importAirspace} onRemove={removeAirspace} />
</ScreenScaffold>

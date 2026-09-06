<script lang="ts">
  import type { ImportAirspaceResult } from '$lib/client';

  import AirspaceSetting from '$lib/AirspaceSetting.svelte';
  import { getAppContext } from '$lib/app-context';

  const { client, airspace } = getAppContext();

  function importAirspace(): Promise<ImportAirspaceResult> {
    return client.importAirspace();
  }

  function removeAirspace(sourceName: string): Promise<void> {
    return client.removeAirspace(sourceName);
  }
</script>

<AirspaceSetting status={airspace.current} onImport={importAirspace} onRemove={removeAirspace} />

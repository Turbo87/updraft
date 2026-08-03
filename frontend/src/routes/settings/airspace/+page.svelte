<script lang="ts">
  import type { ImportAirspaceResult } from '$lib/client';

  import { resolve } from '$app/paths';

  import AirspaceSetting from '$lib/AirspaceSetting.svelte';
  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';

  const { client, airspace } = getAppContext();

  function importAirspace(): Promise<ImportAirspaceResult> {
    return client.importAirspace();
  }

  function removeAirspace(): Promise<void> {
    return client.removeAirspace();
  }
</script>

<main>
  <a class="back-link" href={resolve('/settings')}>{m.back_to_settings()}</a>
  <h1>{m.airspace_label()}</h1>
  <AirspaceSetting status={airspace.current} onImport={importAirspace} onRemove={removeAirspace} />
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  .back-link {
    display: inline-block;
    margin-block-end: 1rem;
  }

  h1 {
    margin-block-start: 0;
  }
</style>

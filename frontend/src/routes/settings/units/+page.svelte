<script lang="ts">
  import type { UnitSettings as UnitSettingsValue } from '$lib/protocol/generated/UnitSettings';

  import { resolve } from '$app/paths';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import UnitSettings from '$lib/UnitSettings.svelte';

  const { client, settings } = getAppContext();
  let optimisticUnits = $state.raw<UnitSettingsValue | null>(null);
  const activeUnits = $derived(optimisticUnits ?? settings.current.units);

  function selectUnits(units: UnitSettingsValue): void {
    optimisticUnits = units;
    void client
      .setUnits(units)
      .catch((error: unknown) => {
        console.error('Failed to set units', error);
      })
      .finally(() => {
        if (optimisticUnits === units) optimisticUnits = null;
      });
  }
</script>

<main>
  <a class="back-link" href={resolve('/settings')}>{m.back_to_settings()}</a>
  <h1>{m.units_label()}</h1>
  <UnitSettings units={activeUnits} onUnitsChange={selectUnits} />
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

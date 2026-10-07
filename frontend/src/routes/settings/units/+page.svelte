<script lang="ts">
  import type { UnitSettings as UnitSettingsValue } from '#lib/protocol/generated/UnitSettings.js';

  import { resolve } from '$app/paths';

  import { getAppContext } from '#lib/app-context.js';
  import { m } from '#lib/paraglide/messages.js';
  import ScreenScaffold from '#lib/ScreenScaffold.svelte';
  import UnitSettings from '#lib/UnitSettings.svelte';

  const { client, settings } = getAppContext();
  let optimisticUnits = $state.raw<UnitSettingsValue | null>(null);
  const activeUnits = $derived(optimisticUnits ?? settings.current.units);

  function selectUnits(units: UnitSettingsValue): void {
    optimisticUnits = units;
    void client
      .changeSetting({ type: 'units', units })
      .catch((error: unknown) => {
        console.error('Failed to set units', error);
      })
      .finally(() => {
        if (optimisticUnits === units) optimisticUnits = null;
      });
  }
</script>

<ScreenScaffold
  backHref={resolve('/settings')}
  backLabel={m.back_to_settings()}
  title={m.units_label()}
>
  <UnitSettings units={activeUnits} onUnitsChange={selectUnits} />
</ScreenScaffold>

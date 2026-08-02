<script lang="ts">
  import type { ImportAirspaceResult } from '$lib/client';
  import type { Locale } from '$lib/protocol/generated/Locale';
  import type { UnitSettings as UnitSettingsValue } from '$lib/protocol/generated/UnitSettings';

  import { resolve } from '$app/paths';

  import AirspaceSetting from '$lib/AirspaceSetting.svelte';
  import { getAppContext } from '$lib/app-context';
  import LanguageSetting from '$lib/LanguageSetting.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import UnitSettings from '$lib/UnitSettings.svelte';

  const { client, airspace, settings } = getAppContext();
  const activeLocale = $derived(settings.current.locale ?? getLocale());
  let optimisticUnits = $state.raw<UnitSettingsValue | null>(null);
  const activeUnits = $derived(optimisticUnits ?? settings.current.units);

  function selectLocale(locale: Locale): void {
    void client.setLocale(locale).catch((error: unknown) => {
      console.error('Failed to set locale', error);
    });
  }

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

  function importAirspace(): Promise<ImportAirspaceResult> {
    return client.importAirspace();
  }

  function removeAirspace(): Promise<void> {
    return client.removeAirspace();
  }
</script>

<main>
  <h1>{m.settings_heading()}</h1>
  <div class="settings">
    <LanguageSetting locale={activeLocale} onLocaleChange={selectLocale} />
    <UnitSettings units={activeUnits} onUnitsChange={selectUnits} />
    <AirspaceSetting
      status={airspace.current}
      onImport={importAirspace}
      onRemove={removeAirspace}
    />
  </div>
  <a href={resolve('/devices')}>{m.external_devices_heading()}</a>
  <a href={resolve('/')}>{m.back_to_flight_view()}</a>
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  h1 {
    margin-block-start: 0;
  }

  .settings {
    display: grid;
    gap: 2rem;
  }

  a {
    display: inline-block;
    margin-block-start: 2rem;
  }

  a + a {
    margin-inline-start: 1rem;
  }
</style>

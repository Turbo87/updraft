<script lang="ts">
  import ListRow from './ListRow.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    language?: string;
    buildDate?: string;
  };

  let { language, buildDate }: Props = $props();
</script>

<ScreenScaffold backHref="/" backLabel={m.back_to_flight_view()} title={m.settings_heading()}>
  <nav aria-label={m.settings_heading()}>
    <ListRow
      href="/settings/language"
      icon="i-mdi-translate"
      label={m.language_label()}
      size="large"
      value={language ?? '—'}
    />
    <ListRow href="/settings/units" icon="i-mdi-ruler" label={m.units_label()} size="large" />
    <ListRow
      href="/settings/airspace"
      icon="i-mdi-vector-square"
      label={m.airspace_label()}
      size="large"
    />
    <ListRow
      href="/settings/devices"
      icon="i-mdi-lan-connect"
      label={m.external_devices_heading()}
      size="large"
    />
    <ListRow
      href="/settings/about"
      icon="i-mdi-information-outline"
      label={m.about_heading()}
      size="large"
      value={buildDate ?? '—'}
    />
  </nav>
</ScreenScaffold>

<style>
  nav {
    display: grid;
    gap: var(--space-2);
  }
</style>

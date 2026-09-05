<script lang="ts">
  import Button from './Button.svelte';
  import ListRow from './ListRow.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    language?: string;
    buildDate?: string;
    onQuit?: () => void;
  };

  let { language, buildDate, onQuit }: Props = $props();
</script>

<ScreenScaffold backHref="/" backLabel={m.back_to_flight_view()} title={m.settings_heading()}>
  <nav aria-label={m.settings_heading()}>
    <ListRow
      href="/settings/flight-controls"
      icon="i-mdi-tune"
      label={m.flight_controls_heading()}
      size="large"
    />
    <ListRow
      href="/settings/language"
      icon="i-mdi-translate"
      label={m.language_label()}
      size="large"
      value={language ?? '—'}
    />
    <ListRow
      href="/settings/waypoints"
      icon="i-mdi-map-marker"
      label={m.waypoints_heading()}
      size="large"
    />
    <ListRow href="/settings/units" icon="i-mdi-ruler" label={m.units_label()} size="large" />
    <ListRow href="/settings/glide" icon="i-mdi-airplane" label={m.glide_heading()} size="large" />
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
  {#if onQuit}
    <section class="quit-action">
      <p>{m.quit_app_hint()}</p>
      <Button size="large" style="width: 100%" variant="destructive-outline" onclick={onQuit}>
        <span aria-hidden="true" class="i-mdi-power action-icon"></span>
        {m.quit_app()}
      </Button>
    </section>
  {/if}
</ScreenScaffold>

<style>
  nav {
    display: grid;
    gap: var(--space-2);
  }

  .quit-action {
    margin-block-start: var(--space-8);
  }

  .quit-action p {
    margin: 0 0 var(--space-3);
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  .action-icon {
    font-size: 1.25rem;
  }
</style>

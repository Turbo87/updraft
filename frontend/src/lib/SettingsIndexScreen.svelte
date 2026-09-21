<script lang="ts">
  import Button from './Button.svelte';
  import Card from './Card.svelte';
  import ListRow from './ListRow.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    language?: string;
    buildDate?: string;
    updateCount?: number;
    onQuit?: () => void;
  };

  let { language, buildDate, updateCount = 0, onQuit }: Props = $props();
</script>

<ScreenScaffold backHref="/" backLabel={m.back_to_flight_view()} title={m.settings_heading()}>
  <nav aria-label={m.settings_heading()}>
    <Card>
      <div class="group">
        <ListRow
          href="/navigation"
          icon="i-mdi-navigation"
          label={m.navigation_heading()}
          size="large"
        />
        <ListRow
          href="/settings/flight-controls"
          icon="i-mdi-tune"
          label={m.flight_controls_heading()}
          size="large"
        />
        <ListRow
          href="/settings/map"
          icon="i-mdi-map-outline"
          label={m.map_settings_heading()}
          size="large"
        />
        <ListRow
          href="/settings/glide"
          icon="i-mdi-airplane"
          label={m.glide_heading()}
          size="large"
        />
        <ListRow
          href="/settings/vario"
          icon="i-mdi-airplane-marker"
          label={m.vario_settings_heading()}
          size="large"
        />
        <ListRow
          href="/settings/traffic"
          icon="i-mdi-radar"
          label={m.traffic_settings_heading()}
          size="large"
        />
      </div>
    </Card>
    <Card>
      <div class="group">
        <ListRow
          class="data-settings"
          href="/settings/data"
          icon="i-mdi-database-outline"
          label={m.data_heading()}
          size="large"
          value={updateCount === 1
            ? m.data_update_count_one()
            : updateCount > 0
              ? m.data_update_count({ count: updateCount })
              : ''}
        />
        <ListRow
          href="/settings/devices"
          icon="i-mdi-lan-connect"
          label={m.external_devices_heading()}
          size="large"
        />
      </div>
    </Card>
    <Card>
      <div class="group">
        <ListRow
          href="/settings/language"
          icon="i-mdi-translate"
          label={m.language_label()}
          size="large"
          value={language ?? '—'}
        />
        <ListRow href="/settings/units" icon="i-mdi-ruler" label={m.units_label()} size="large" />
        <ListRow
          href="/settings/about"
          icon="i-mdi-information-outline"
          label={m.about_heading()}
          size="large"
          value={buildDate ?? '—'}
        />
      </div>
    </Card>
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
  nav :global(.data-settings .value) {
    max-width: 30vw;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  nav {
    display: grid;
    gap: var(--space-6);
  }

  .group > :global(.list-row) {
    border-radius: 0;
  }

  .group > :global(.list-row + .list-row) {
    border-block-start: 1px solid var(--color-separator);
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

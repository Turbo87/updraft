<script lang="ts">
  import { resolve } from '$app/paths';

  import Card from './Card.svelte';
  import ListRow from './ListRow.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    language?: string;
    updateCount?: number;
  };

  let { language, updateCount = 0 }: Props = $props();
</script>

<ScreenScaffold
  backHref={resolve('/menu')}
  backLabel={m.back_to_main_menu()}
  title={m.settings_heading()}
>
  <nav aria-label={m.settings_heading()}>
    <Card>
      <div class="group">
        <ListRow
          href={resolve('/settings/map')}
          icon="i-mdi-map-outline"
          label={m.map_settings_heading()}
          size="large"
        />
        <ListRow
          href={resolve('/settings/glide')}
          icon="i-mdi-airplane"
          label={m.glide_heading()}
          size="large"
        />
        <ListRow
          href={resolve('/settings/vario')}
          icon="i-mdi-airplane-marker"
          label={m.vario_settings_heading()}
          size="large"
        />
        <ListRow
          href={resolve('/settings/traffic')}
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
          href={resolve('/settings/data')}
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
          href={resolve('/settings/devices')}
          icon="i-mdi-lan-connect"
          label={m.external_devices_heading()}
          size="large"
        />
      </div>
    </Card>
    <Card>
      <div class="group">
        <ListRow
          href={resolve('/settings/language')}
          icon="i-mdi-translate"
          label={m.language_label()}
          size="large"
          value={language ?? '—'}
        />
        <ListRow
          href={resolve('/settings/units')}
          icon="i-mdi-ruler"
          label={m.units_label()}
          size="large"
        />
      </div>
    </Card>
  </nav>
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
</style>

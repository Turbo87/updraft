<script lang="ts">
  import { resolve } from '$app/paths';

  import Button from './Button.svelte';
  import Card from './Card.svelte';
  import ListRow from './ListRow.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type Props = {
    buildDate?: string;
    onQuit?: () => void;
  };

  let { buildDate, onQuit }: Props = $props();
</script>

<ScreenScaffold
  backHref={resolve('/')}
  backLabel={m.back_to_flight_view()}
  title={m.main_menu_heading()}
>
  <nav aria-label={m.main_menu_heading()}>
    <Card>
      <div class="group">
        <ListRow
          href={resolve('/navigation')}
          icon="i-mdi-navigation"
          label={m.navigation_heading()}
          size="large"
        />
        <ListRow
          href={resolve('/flight-controls')}
          icon="i-mdi-tune"
          label={m.flight_controls_heading()}
          size="large"
        />
      </div>
    </Card>
    <Card>
      <div class="group">
        <ListRow
          href={resolve('/settings')}
          icon="i-mdi-cog-outline"
          label={m.settings_heading()}
          size="large"
        />
        <ListRow
          href={resolve('/about')}
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

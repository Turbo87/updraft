<script lang="ts">
  import type { InfoboxField } from './fields';

  import { m } from '$lib/paraglide/messages';
  import Infobox from './Infobox.svelte';

  type Props = { infoboxes: InfoboxField[] };
  let { infoboxes }: Props = $props();
</script>

<section class="infobox-dock" aria-label={m.flight_view_instruments()}>
  <div class="cells">
    {#each infoboxes as infobox (infobox.id)}
      <Infobox label={infobox.label} value={infobox.value} stale={infobox.stale} />
    {/each}
  </div>
</section>

<style>
  .infobox-dock {
    flex: none;
    width: 100%;
    height: calc(9rem + var(--safe-area-bottom));
    padding: 0 var(--safe-area-right) var(--safe-area-bottom) var(--safe-area-left);
    background: var(--color-screen-surface);
  }

  .cells {
    display: grid;
    width: 100%;
    height: 100%;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    grid-template-rows: repeat(2, minmax(0, 1fr));
    gap: 1px;
    border: 1px solid var(--color-separator);
    background: var(--color-separator);
  }

  @media (orientation: landscape) {
    .infobox-dock {
      width: calc(10.5rem + var(--safe-area-right));
      height: 100%;
      padding: var(--safe-area-top) var(--safe-area-right) var(--safe-area-bottom) 0;
    }

    .cells {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      grid-template-rows: repeat(5, minmax(0, 1fr));
    }
  }
</style>

<script lang="ts">
  import type { PinnedTarget } from '$lib/protocol/generated/PinnedTarget';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { navigationLabel } from '$lib/navigation';
  import { m } from '$lib/paraglide/messages';
  import TargetBar from './TargetBar.svelte';

  type Props = { pins: PinnedTarget[]; units: UnitSettings; hasPrimary: boolean };
  let { pins, units, hasPrimary }: Props = $props();
  const visible = $derived(pins.filter((pin) => !pin.primary));
</script>

{#if visible.length}
  <section aria-label={m.pins_heading()} class:standalone={!hasPrimary}>
    {#each visible as pin (pin.id)}
      <TargetBar
        navigation={pin.navigation}
        {units}
        compact
        href={`/pinned-targets/${pin.id}`}
        label={`${m.pins_details()}: ${navigationLabel(pin.navigation)}`}
      />
    {/each}
  </section>
{/if}

<style>
  section {
    flex: 0 1 auto;
    min-height: 0;
    max-height: 30dvh;
    overflow-y: auto;
    background: var(--color-screen-surface);
    overscroll-behavior: contain;
  }
  .standalone {
    padding-top: var(--safe-area-top);
  }
</style>

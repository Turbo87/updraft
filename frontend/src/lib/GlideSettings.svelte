<script lang="ts">
  import type { PolarId } from '$lib/protocol/generated/PolarId';

  import { onMount } from 'svelte';

  import Button from './Button.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';

  let {
    polar,
    getPolars,
    setPolar,
  }: {
    polar: PolarId;
    getPolars: () => Promise<PolarId[]>;
    setPolar: (polar: PolarId) => Promise<void>;
  } = $props();

  let catalog = $state.raw<PolarId[] | null>(null);
  let loadingFailed = $state(false);
  let saving = $state(false);
  let saveFailed = $state(false);

  onMount(() => {
    void loadPolars();
  });

  async function loadPolars() {
    loadingFailed = false;
    try {
      catalog = await getPolars();
    } catch {
      loadingFailed = true;
    }
  }

  async function selectPolar(event: Event & { currentTarget: HTMLSelectElement }) {
    let select = event.currentTarget;
    saving = true;
    saveFailed = false;
    try {
      await setPolar(select.value);
    } catch {
      saveFailed = true;
    } finally {
      select.value = polar;
      saving = false;
    }
  }
</script>

<ScreenScaffold backHref="/settings" backLabel={m.back_to_settings()} title={m.glide_heading()}>
  {#if loadingFailed}
    <p role="alert">{m.polars_load_failed()}</p>
    <Button onclick={loadPolars}>{m.retry()}</Button>
  {:else if catalog}
    <label>
      <span>{m.polar_label()}</span>
      <select value={polar} disabled={saving} onchange={selectPolar}>
        {#each catalog as name (name)}
          <option value={name}>{name}</option>
        {/each}
      </select>
    </label>
    {#if saveFailed}<p role="alert">{m.polar_save_failed()}</p>{/if}
  {:else}
    <p role="status">{m.polars_loading()}</p>
  {/if}
</ScreenScaffold>

<style>
  label {
    display: grid;
    gap: var(--space-2);
  }
  label span {
    color: var(--color-text-muted);
    font: var(--text-row-label);
  }
  select {
    width: 100%;
    min-height: var(--target-min);
    padding-inline: 0.875rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-control);
    background: var(--color-screen-surface);
    color: var(--color-text);
    font: var(--text-input);
  }
  select:focus-visible {
    outline: 2px solid var(--color-focus-ring);
  }
</style>

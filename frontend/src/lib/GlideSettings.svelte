<script lang="ts">
  import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
  import type { PolarId } from '$lib/protocol/generated/PolarId';

  import { onMount } from 'svelte';

  import Button from './Button.svelte';
  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import { convertAltitude } from './units';

  type Props = {
    polar: PolarId;
    getPolars: () => Promise<PolarId[]>;
    setPolar: (polar: PolarId) => Promise<void>;
    arrivalReserve: number;
    altitudeUnit: AltitudeUnit;
    setArrivalReserve: (reserve: number) => Promise<void>;
  };

  let { polar, getPolars, setPolar, arrivalReserve, altitudeUnit, setArrivalReserve }: Props =
    $props();

  let catalog = $state.raw<PolarId[] | null>(null);
  let loadingFailed = $state(false);
  let saving = $state(false);
  let saveFailed = $state(false);
  let reserveSaving = $state(false);
  let reserveError = $state<'invalid' | 'failed' | null>(null);
  let displayedReserve = $derived(convertAltitude(arrivalReserve, altitudeUnit).toFixed(0));

  async function changeReserve(event: Event & { currentTarget: HTMLInputElement }) {
    let input = event.currentTarget;
    let reserve = input.valueAsNumber / convertAltitude(1, altitudeUnit);
    if (!Number.isFinite(reserve) || reserve < 0) {
      reserveError = 'invalid';
      return;
    }
    reserveSaving = true;
    reserveError = null;
    try {
      await setArrivalReserve(reserve);
    } catch {
      reserveError = 'failed';
    } finally {
      input.value = displayedReserve;
      reserveSaving = false;
    }
  }

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
  <label class="reserve">
    <span>{m.arrival_reserve_label()} ({altitudeUnit})</span>
    <input
      type="number"
      min="0"
      step="any"
      value={displayedReserve}
      disabled={reserveSaving}
      aria-invalid={reserveError ? 'true' : undefined}
      onchange={changeReserve}
    />
  </label>
  {#if reserveError}
    <p role="alert">
      {reserveError === 'invalid' ? m.arrival_reserve_invalid() : m.arrival_reserve_save_failed()}
    </p>
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
  .reserve {
    margin-top: var(--space-6);
  }
  select,
  input {
    box-sizing: border-box;
    width: 100%;
    min-height: var(--target-min);
    padding-inline: 0.875rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-control);
    background: var(--color-screen-surface);
    color: var(--color-text);
    font: var(--text-input);
  }
  select:focus-visible,
  input:focus-visible {
    outline: 2px solid var(--color-focus-ring);
  }
</style>

<script lang="ts">
  import type { VerticalSpeedUnit } from './units';

  import { m } from './paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import { convertVerticalSpeed } from './units';

  let {
    macCready,
    unit,
    setMacCready,
    bugs,
    setBugs,
  }: {
    macCready: number;
    unit: VerticalSpeedUnit;
    setMacCready: (value: number) => Promise<void>;
    bugs: number;
    setBugs: (value: number) => Promise<void>;
  } = $props();

  let saving = $state(false);
  let error = $state<'invalid' | 'failed' | null>(null);
  let bugsSaving = $state(false);
  let bugsError = $state<'invalid' | 'failed' | null>(null);
  let displayedMC = $derived(
    convertVerticalSpeed(macCready, unit).toFixed(unit === 'ft/min' ? 0 : 1),
  );

  async function changeMC(event: Event & { currentTarget: HTMLInputElement }) {
    let input = event.currentTarget;
    let value = input.valueAsNumber / convertVerticalSpeed(1, unit);
    if (!Number.isFinite(value) || value < 0) {
      error = 'invalid';
      return;
    }
    saving = true;
    error = null;
    try {
      await setMacCready(value);
    } catch {
      error = 'failed';
    } finally {
      input.value = displayedMC;
      saving = false;
    }
  }

  async function changeBugs(event: Event & { currentTarget: HTMLInputElement }) {
    let input = event.currentTarget;
    let value = input.valueAsNumber;
    if (!Number.isFinite(value) || value < 0 || value >= 100) {
      bugsError = 'invalid';
      return;
    }
    bugsSaving = true;
    bugsError = null;
    try {
      await setBugs(value);
    } catch {
      bugsError = 'failed';
    } finally {
      input.value = String(bugs);
      bugsSaving = false;
    }
  }
</script>

<ScreenScaffold
  backHref="/settings"
  backLabel={m.back_to_settings()}
  title={m.flight_controls_heading()}
>
  <p>{m.flight_controls_reset_hint()}</p>
  <label>
    <span>MC ({unit})</span>
    <input
      type="number"
      min="0"
      step="any"
      value={displayedMC}
      disabled={saving}
      aria-invalid={error ? 'true' : undefined}
      onchange={changeMC}
    />
  </label>
  {#if error}<p role="alert">{error === 'invalid' ? m.mc_invalid() : m.mc_save_failed()}</p>{/if}
  <label class="bugs">
    <span>{m.bugs_label()} (%)</span>
    <input
      type="number"
      min="0"
      step="any"
      value={bugs}
      disabled={bugsSaving}
      aria-invalid={bugsError ? 'true' : undefined}
      onchange={changeBugs}
    />
  </label>
  <p>{m.bugs_hint()}</p>
  {#if bugsError}
    <p role="alert">
      {bugsError === 'invalid' ? m.bugs_invalid() : m.bugs_save_failed()}
    </p>
  {/if}
</ScreenScaffold>

<style>
  .bugs {
    margin-block-start: var(--space-6);
  }
  p {
    font: var(--text-caption);
    color: var(--color-text-muted);
  }
  label {
    display: grid;
    gap: var(--space-2);
  }
  label span {
    color: var(--color-text-muted);
    font: var(--text-row-label);
  }
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
  input:focus-visible {
    outline: 2px solid var(--color-focus-ring);
  }
</style>

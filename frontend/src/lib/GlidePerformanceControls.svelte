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
    ballast,
    setBallast,
  }: {
    macCready: number;
    unit: VerticalSpeedUnit;
    setMacCready: (value: number) => Promise<void>;
    bugs: number;
    setBugs: (value: number) => Promise<void>;
    ballast: number;
    setBallast: (value: number) => Promise<void>;
  } = $props();

  let saving = $state(false);
  let error = $state<'invalid' | 'failed' | null>(null);
  let bugsSaving = $state(false);
  let bugsError = $state<'invalid' | 'failed' | null>(null);
  let ballastSaving = $state(false);
  let ballastError = $state<'invalid' | 'failed' | null>(null);
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

  async function changeBallast(event: Event & { currentTarget: HTMLInputElement }) {
    let input = event.currentTarget;
    let value = input.valueAsNumber;
    if (!Number.isFinite(value) || value < 0) {
      ballastError = 'invalid';
      return;
    }
    ballastSaving = true;
    ballastError = null;
    try {
      await setBallast(value);
    } catch {
      ballastError = 'failed';
    } finally {
      input.value = String(ballast);
      ballastSaving = false;
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
      readonly={saving}
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
      readonly={bugsSaving}
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
  <label class="ballast">
    <span>{m.ballast_label()} (L)</span>
    <input
      type="number"
      min="0"
      step="any"
      value={ballast}
      readonly={ballastSaving}
      aria-invalid={ballastError ? 'true' : undefined}
      onchange={changeBallast}
    />
  </label>
  <p>{m.ballast_hint()}</p>
  {#if ballastError}
    <p role="alert">
      {ballastError === 'invalid' ? m.ballast_invalid() : m.ballast_save_failed()}
    </p>
  {/if}
</ScreenScaffold>

<style>
  .bugs,
  .ballast {
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

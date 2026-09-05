<script lang="ts">
  import type { VerticalSpeedUnit } from './units';

  import { m } from './paraglide/messages.js';
  import { convertVerticalSpeed } from './units';

  let {
    macCready,
    unit,
    setMacCready,
  }: {
    macCready: number;
    unit: VerticalSpeedUnit;
    setMacCready: (value: number) => Promise<void>;
  } = $props();

  let saving = $state(false);
  let error = $state<'invalid' | 'failed' | null>(null);
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
</script>

<section>
  <h2>{m.flight_controls_heading()}</h2>
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
</section>

<style>
  section {
    margin-block-end: var(--space-6);
  }
  h2 {
    font: var(--text-row-label);
    margin: 0;
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

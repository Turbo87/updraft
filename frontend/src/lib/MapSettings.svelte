<script lang="ts">
  import type { HillshadeDirection } from '#lib/protocol/generated/HillshadeDirection.js';
  import type { TrailColorMode } from '#lib/protocol/generated/TrailColorMode.js';
  import type { TrailLength } from '#lib/protocol/generated/TrailLength.js';

  import { m } from '#lib/paraglide/messages.js';
  import AsyncRadioList from './AsyncRadioList.svelte';
  import Button from './Button.svelte';

  const MAX_TRAIL_LENGTH_MINUTES = 24 * 60;

  type Props = {
    direction: HillshadeDirection;
    setDirection: (direction: HillshadeDirection) => Promise<void>;
    trailColorMode: TrailColorMode;
    setTrailColorMode: (mode: TrailColorMode) => Promise<void>;
    trailLength: TrailLength;
    setTrailLength: (length: TrailLength) => Promise<void>;
  };

  let {
    direction,
    setDirection,
    trailColorMode,
    setTrailColorMode,
    trailLength,
    setTrailLength,
  }: Props = $props();
  let options = $derived([
    { value: 'fixed', label: m.hillshade_direction_fixed() },
    { value: 'wind', label: m.hillshade_direction_wind() },
    { value: 'sun', label: m.hillshade_direction_sun() },
  ] satisfies ReadonlyArray<{ value: HillshadeDirection; label: string }>);
  let trailColorOptions = $derived([
    { value: 'fixed', label: m.trail_color_mode_fixed() },
    { value: 'altitude', label: m.trail_color_mode_altitude() },
    { value: 'agl', label: m.trail_color_mode_agl() },
    { value: 'vario', label: m.trail_color_mode_vario() },
    { value: 'netto', label: m.trail_color_mode_netto() },
    { value: 'relativeVario', label: m.trail_color_mode_relative_vario() },
  ] satisfies ReadonlyArray<{ value: TrailColorMode; label: string }>);
  let trailLengthPresets = $derived([
    { value: 0, label: m.trail_length_off() },
    ...[15, 60].map((minutes) => ({ value: minutes, label: m.trail_length_minutes({ minutes }) })),
    ...[3, 5, 10].map((hours) => ({ value: hours * 60, label: m.trail_length_hours({ hours }) })),
  ]);
  let trailLengthInput = $state<HTMLInputElement>();
  let trailLengthSaving = $state(false);
  let trailLengthError = $state<'invalid' | 'failed' | null>(null);

  async function changeTrailLength(length: number) {
    if (!Number.isInteger(length) || length < 0 || length > MAX_TRAIL_LENGTH_MINUTES) {
      trailLengthError = 'invalid';
      return;
    }
    trailLengthSaving = true;
    trailLengthError = null;
    try {
      await setTrailLength(length);
    } catch {
      trailLengthError = 'failed';
    } finally {
      if (trailLengthInput) trailLengthInput.value = String(trailLength);
      trailLengthSaving = false;
    }
  }
</script>

<div class="settings">
  <AsyncRadioList
    name="hillshade-direction"
    legend={m.hillshade_direction_label()}
    {options}
    value={direction}
    onChange={setDirection}
    errorMessage={m.hillshade_direction_change_failed}
  />
  <AsyncRadioList
    name="trail-color-mode"
    legend={m.trail_color_mode_label()}
    options={trailColorOptions}
    value={trailColorMode}
    onChange={setTrailColorMode}
    errorMessage={m.trail_color_mode_change_failed}
  />
  <fieldset class="trail-length" disabled={trailLengthSaving}>
    <label>
      <span>{m.trail_length_label()}</span>
      <input
        bind:this={trailLengthInput}
        type="number"
        min="0"
        max={MAX_TRAIL_LENGTH_MINUTES}
        step="1"
        value={trailLength}
        aria-invalid={trailLengthError === 'invalid' ? 'true' : undefined}
        onchange={(event) => changeTrailLength(event.currentTarget.valueAsNumber)}
      />
    </label>
    <div class="presets">
      {#each trailLengthPresets as preset (preset.value)}
        <Button variant="secondary" onclick={() => changeTrailLength(preset.value)}>
          {preset.label}
        </Button>
      {/each}
    </div>
    {#if trailLengthError}
      <p role="alert">
        {trailLengthError === 'invalid' ? m.trail_length_invalid() : m.trail_length_change_failed()}
      </p>
    {/if}
  </fieldset>
</div>

<style>
  .settings {
    display: grid;
    gap: var(--space-5);
  }
  .trail-length {
    display: grid;
    gap: var(--space-2);
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
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
  .presets {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
  }
</style>

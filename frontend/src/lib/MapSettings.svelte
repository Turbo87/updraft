<script lang="ts">
  import type { HillshadeDirection } from '#lib/protocol/generated/HillshadeDirection.js';
  import type { TrailColorMode } from '#lib/protocol/generated/TrailColorMode.js';

  import { m } from '#lib/paraglide/messages.js';
  import AsyncRadioList from './AsyncRadioList.svelte';

  type Props = {
    direction: HillshadeDirection;
    setDirection: (direction: HillshadeDirection) => Promise<void>;
    trailColorMode: TrailColorMode;
    setTrailColorMode: (mode: TrailColorMode) => Promise<void>;
  };

  let { direction, setDirection, trailColorMode, setTrailColorMode }: Props = $props();
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
</div>

<style>
  .settings {
    display: grid;
    gap: var(--space-5);
  }
</style>

<script lang="ts">
  import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
  import type { DistanceUnit } from '$lib/protocol/generated/DistanceUnit';
  import type { SpeedUnit } from '$lib/protocol/generated/SpeedUnit';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { VerticalSpeedUnit } from '$lib/protocol/generated/VerticalSpeedUnit';

  import InlineChoiceGroup from '$lib/InlineChoiceGroup.svelte';
  import { m } from '$lib/paraglide/messages.js';

  type UnitSettingsProps = {
    units: UnitSettings;
    onUnitsChange: (units: UnitSettings) => void;
  };

  let { units, onUnitsChange }: UnitSettingsProps = $props();

  const altitudeOptions = [
    { value: 'm', label: 'm' },
    { value: 'ft', label: 'ft' },
  ] satisfies ReadonlyArray<{ value: AltitudeUnit; label: string }>;

  const distanceOptions = [
    { value: 'km', label: 'km' },
    { value: 'mi', label: 'mi' },
    { value: 'nm', label: 'nm' },
  ] satisfies ReadonlyArray<{ value: DistanceUnit; label: string }>;

  const speedOptions = [
    { value: 'km/h', label: 'km/h' },
    { value: 'kt', label: 'kt' },
    { value: 'mph', label: 'mph' },
  ] satisfies ReadonlyArray<{ value: SpeedUnit; label: string }>;

  const verticalSpeedOptions = [
    { value: 'm/s', label: 'm/s' },
    { value: 'kt', label: 'kt' },
    { value: 'ft/min', label: 'ft/min' },
  ] satisfies ReadonlyArray<{ value: VerticalSpeedUnit; label: string }>;
</script>

<div class="selections">
  <InlineChoiceGroup
    name="altitude"
    legend={m.altitude_label()}
    options={altitudeOptions}
    value={units.altitude}
    onChange={(altitude) => onUnitsChange({ ...units, altitude })}
  />
  <InlineChoiceGroup
    name="distance"
    legend={m.distance_label()}
    options={distanceOptions}
    value={units.distance}
    onChange={(distance) => onUnitsChange({ ...units, distance })}
  />
  <InlineChoiceGroup
    name="speed"
    legend={m.speed_label()}
    options={speedOptions}
    value={units.speed}
    onChange={(speed) => onUnitsChange({ ...units, speed })}
  />
  <InlineChoiceGroup
    name="verticalSpeed"
    legend={m.vertical_speed_label()}
    options={verticalSpeedOptions}
    value={units.verticalSpeed}
    onChange={(verticalSpeed) => onUnitsChange({ ...units, verticalSpeed })}
  />
</div>

<style>
  .selections {
    display: grid;
    gap: var(--space-6);
  }
</style>

<script lang="ts">
  import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
  import type { DistanceUnit } from '$lib/protocol/generated/DistanceUnit';
  import type { SpeedUnit } from '$lib/protocol/generated/SpeedUnit';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { VerticalSpeedUnit } from '$lib/protocol/generated/VerticalSpeedUnit';

  import { m } from '$lib/paraglide/messages.js';

  type UnitSettingsProps = {
    units: UnitSettings;
    onUnitsChange: (units: UnitSettings) => void;
  };

  let { units, onUnitsChange }: UnitSettingsProps = $props();

  const altitudeUnits: AltitudeUnit[] = ['m', 'ft'];
  const distanceUnits: DistanceUnit[] = ['km', 'mi', 'nm'];
  const speedUnits: SpeedUnit[] = ['km/h', 'kt', 'mph'];
  const verticalSpeedUnits: VerticalSpeedUnit[] = ['m/s', 'kt', 'ft/min'];
</script>

<fieldset>
  <legend>{m.units_label()}</legend>
  <div class="selections">
    <label>
      <span>{m.altitude_label()}</span>
      <select
        name="altitude"
        value={units.altitude}
        onchange={(event) =>
          onUnitsChange({
            ...units,
            altitude: event.currentTarget.value as AltitudeUnit,
          })}
      >
        {#each altitudeUnits as unit (unit)}
          <option value={unit}>{unit}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>{m.distance_label()}</span>
      <select
        name="distance"
        value={units.distance}
        onchange={(event) =>
          onUnitsChange({
            ...units,
            distance: event.currentTarget.value as DistanceUnit,
          })}
      >
        {#each distanceUnits as unit (unit)}
          <option value={unit}>{unit}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>{m.speed_label()}</span>
      <select
        name="speed"
        value={units.speed}
        onchange={(event) =>
          onUnitsChange({
            ...units,
            speed: event.currentTarget.value as SpeedUnit,
          })}
      >
        {#each speedUnits as unit (unit)}
          <option value={unit}>{unit}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>{m.vertical_speed_label()}</span>
      <select
        name="verticalSpeed"
        value={units.verticalSpeed}
        onchange={(event) =>
          onUnitsChange({
            ...units,
            verticalSpeed: event.currentTarget.value as VerticalSpeedUnit,
          })}
      >
        {#each verticalSpeedUnits as unit (unit)}
          <option value={unit}>{unit}</option>
        {/each}
      </select>
    </label>
  </div>
</fieldset>

<style>
  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-block-end: 0.75rem;
    font-weight: 600;
  }

  .selections {
    display: grid;
    max-width: 20rem;
    gap: 0.75rem;
  }

  label {
    display: grid;
    gap: 0.25rem;
  }

  select {
    min-height: 3rem;
  }
</style>

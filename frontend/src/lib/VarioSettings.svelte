<script lang="ts">
  import type { ClimbAverageMethod } from '$lib/protocol/generated/ClimbAverageMethod';

  import { m } from '$lib/paraglide/messages.js';
  import AsyncRadioList from './AsyncRadioList.svelte';

  type Props = {
    method: ClimbAverageMethod;
    setMethod: (method: ClimbAverageMethod) => Promise<void>;
    energyCompensation: boolean;
    setEnergyCompensation: (enabled: boolean) => Promise<void>;
  };

  let { method, setMethod, energyCompensation, setEnergyCompensation }: Props = $props();
  let options = $derived([
    { value: 'smoothed20s', label: m.climb_smoothed_20s_label() },
    { value: 'normalizedEma', label: m.climb_ema_label() },
    { value: 'average20s', label: m.climb_20s_label() },
    { value: 'average30s', label: m.climb_30s_label() },
  ] satisfies ReadonlyArray<{ value: ClimbAverageMethod; label: string }>);
</script>

<div class="settings">
  <AsyncRadioList
    name="climb-average"
    legend={m.climb_average_label()}
    {options}
    value={method}
    onChange={setMethod}
    errorMessage={m.climb_average_change_failed}
  />

  <AsyncRadioList
    name="energy-compensation"
    legend={m.climb_compensation_label()}
    options={[
      {
        value: 'enabled',
        label: m.climb_compensation_enabled(),
        description: m.climb_compensation_description(),
      },
      { value: 'disabled', label: m.climb_compensation_disabled() },
    ]}
    value={energyCompensation ? 'enabled' : 'disabled'}
    onChange={(value) => setEnergyCompensation(value === 'enabled')}
    errorMessage={m.climb_compensation_change_failed}
  />
</div>

<style>
  .settings {
    display: grid;
    gap: var(--space-5);
  }
</style>

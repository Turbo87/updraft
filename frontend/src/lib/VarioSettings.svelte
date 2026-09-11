<script lang="ts">
  import type { ClimbAverageMethod } from '$lib/protocol/generated/ClimbAverageMethod';

  import { m } from '$lib/paraglide/messages.js';
  import RadioList from './RadioList.svelte';

  type Props = {
    method: ClimbAverageMethod;
    setMethod: (method: ClimbAverageMethod) => Promise<void>;
    energyCompensation: boolean;
    setEnergyCompensation: (enabled: boolean) => Promise<void>;
  };

  let { method, setMethod, energyCompensation, setEnergyCompensation }: Props = $props();
  let pendingCompensation = $state<boolean | null>(null);
  let compensationError = $state<string>();
  let pending = $state<ClimbAverageMethod | null>(null);
  let error = $state<string>();
  let options = $derived([
    { value: 'smoothed20s', label: m.climb_smoothed_20s_label() },
    { value: 'normalizedEma', label: m.climb_ema_label() },
    { value: 'average20s', label: m.climb_20s_label() },
    { value: 'average30s', label: m.climb_30s_label() },
  ] satisfies ReadonlyArray<{ value: ClimbAverageMethod; label: string }>);

  async function selectCompensation(value: string) {
    pendingCompensation = value === 'enabled';
    compensationError = undefined;
    try {
      await setEnergyCompensation(pendingCompensation);
    } catch {
      compensationError = m.climb_compensation_change_failed();
    } finally {
      pendingCompensation = null;
    }
  }

  async function select(method: ClimbAverageMethod) {
    pending = method;
    error = undefined;
    try {
      await setMethod(method);
    } catch {
      error = m.climb_average_change_failed();
    } finally {
      pending = null;
    }
  }
</script>

<div class="settings">
  <RadioList
    name="climb-average"
    legend={m.climb_average_label()}
    {options}
    value={pending ?? method}
    disabled={pending !== null}
    {error}
    onChange={select}
  />

  <RadioList
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
    value={(pendingCompensation ?? energyCompensation) ? 'enabled' : 'disabled'}
    disabled={pendingCompensation !== null}
    error={compensationError}
    onChange={selectCompensation}
  />
</div>

<style>
  .settings {
    display: grid;
    gap: var(--space-5);
  }
</style>

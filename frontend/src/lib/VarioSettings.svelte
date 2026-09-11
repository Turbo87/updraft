<script lang="ts">
  import type { ClimbAverageMethod } from '$lib/protocol/generated/ClimbAverageMethod';

  import { m } from '$lib/paraglide/messages.js';
  import RadioList from './RadioList.svelte';

  type Props = {
    method: ClimbAverageMethod;
    setMethod: (method: ClimbAverageMethod) => Promise<void>;
  };

  let { method, setMethod }: Props = $props();
  let pending = $state<ClimbAverageMethod | null>(null);
  let error = $state<string>();
  let options = $derived([
    { value: 'smoothed20s', label: m.climb_smoothed_20s_label() },
    { value: 'normalizedEma', label: m.climb_ema_label() },
    { value: 'average20s', label: m.climb_20s_label() },
    { value: 'average30s', label: m.climb_30s_label() },
  ] satisfies ReadonlyArray<{ value: ClimbAverageMethod; label: string }>);

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

<RadioList
  name="climb-average"
  legend={m.climb_average_label()}
  {options}
  value={pending ?? method}
  disabled={pending !== null}
  {error}
  onChange={select}
/>

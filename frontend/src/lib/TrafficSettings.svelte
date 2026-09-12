<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import RadioList from './RadioList.svelte';

  type Props = {
    enabled: boolean;
    setEnabled: (enabled: boolean) => Promise<void>;
  };

  let { enabled, setEnabled }: Props = $props();
  let pending = $state<boolean | null>(null);
  let error = $state<string>();

  async function select(value: string) {
    pending = value === 'enabled';
    error = undefined;
    try {
      await setEnabled(pending);
    } catch {
      error = m.flarm_correction_change_failed();
    } finally {
      pending = null;
    }
  }
</script>

<RadioList
  name="flarm-position-correction"
  legend={m.flarm_correction_label()}
  options={[
    {
      value: 'enabled',
      label: m.flarm_correction_enabled(),
      description: m.flarm_correction_description(),
    },
    { value: 'disabled', label: m.flarm_correction_disabled() },
  ]}
  value={(pending ?? enabled) ? 'enabled' : 'disabled'}
  disabled={pending !== null}
  {error}
  onChange={select}
/>

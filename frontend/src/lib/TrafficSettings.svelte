<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import AsyncRadioList from './AsyncRadioList.svelte';

  type Props = {
    enabled: boolean;
    setEnabled: (enabled: boolean) => Promise<void>;
  };

  let { enabled, setEnabled }: Props = $props();
</script>

<AsyncRadioList
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
  value={enabled ? 'enabled' : 'disabled'}
  onChange={(value) => setEnabled(value === 'enabled')}
  errorMessage={m.flarm_correction_change_failed}
/>

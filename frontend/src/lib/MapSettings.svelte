<script lang="ts">
  import type { HillshadeDirection } from '$lib/protocol/generated/HillshadeDirection';

  import { m } from '$lib/paraglide/messages.js';
  import AsyncRadioList from './AsyncRadioList.svelte';

  type Props = {
    direction: HillshadeDirection;
    setDirection: (direction: HillshadeDirection) => Promise<void>;
  };

  let { direction, setDirection }: Props = $props();
  let options = $derived([
    { value: 'fixed', label: m.hillshade_direction_fixed() },
    { value: 'wind', label: m.hillshade_direction_wind() },
    { value: 'sun', label: m.hillshade_direction_sun() },
  ] satisfies ReadonlyArray<{ value: HillshadeDirection; label: string }>);
</script>

<AsyncRadioList
  name="hillshade-direction"
  legend={m.hillshade_direction_label()}
  {options}
  value={direction}
  onChange={setDirection}
  errorMessage={m.hillshade_direction_change_failed}
/>

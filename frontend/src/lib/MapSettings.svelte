<script lang="ts">
  import type { HillshadeDirection } from '$lib/protocol/generated/HillshadeDirection';

  import { m } from '$lib/paraglide/messages.js';
  import RadioList from './RadioList.svelte';

  type Props = {
    direction: HillshadeDirection;
    setDirection: (direction: HillshadeDirection) => Promise<void>;
  };

  let { direction, setDirection }: Props = $props();
  let pending = $state<HillshadeDirection | null>(null);
  let error = $state<string>();
  let options = $derived([
    { value: 'fixed', label: m.hillshade_direction_fixed() },
    { value: 'wind', label: m.hillshade_direction_wind() },
    { value: 'sun', label: m.hillshade_direction_sun() },
  ] satisfies ReadonlyArray<{ value: HillshadeDirection; label: string }>);

  async function select(direction: HillshadeDirection) {
    pending = direction;
    error = undefined;
    try {
      await setDirection(direction);
    } catch {
      error = m.hillshade_direction_change_failed();
    } finally {
      pending = null;
    }
  }
</script>

<RadioList
  name="hillshade-direction"
  legend={m.hillshade_direction_label()}
  {options}
  value={pending ?? direction}
  disabled={pending !== null}
  {error}
  onChange={select}
/>

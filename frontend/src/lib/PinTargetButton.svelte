<script lang="ts">
  import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import { targetsMatch } from '$lib/navigation-target';
  import { m } from '$lib/paraglide/messages';

  type Props = { target: NavigationTarget };
  let { target }: Props = $props();
  const { client, navigation } = getAppContext();
  const pin = $derived(navigation.pins.find((pin) => targetsMatch(pin.navigation.target, target)));
  let busy = $state(false);
  let retry = $state.raw<
    { id: number; target: NavigationTarget } | { target: NavigationTarget } | null
  >(null);

  const pending = $derived(retry && targetsMatch(retry.target, target) ? retry : null);

  async function change() {
    let operation = pending ?? (pin ? { id: pin.id, target } : { target });
    busy = true;
    try {
      let saved =
        'id' in operation
          ? await client.unpinTarget(operation.id)
          : await client.pinTarget(operation.target);
      retry = saved ? null : operation;
    } catch {
      retry = operation;
    } finally {
      busy = false;
    }
  }
</script>

<div>
  {#if pending}<p role="alert">{m.pins_save_failed()}</p>{/if}
  <Button onclick={change} loading={busy}
    >{pending ? m.retry() : pin ? m.pins_unpin() : m.pins_pin()}</Button
  >
</div>

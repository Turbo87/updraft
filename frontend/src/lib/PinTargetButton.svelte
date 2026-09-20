<script lang="ts">
  import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import { targetsMatch } from '$lib/navigation-target';
  import { m } from '$lib/paraglide/messages';

  type Props = {
    target: NavigationTarget;
    iconOnly?: boolean;
    onFailure?: (retry: () => Promise<boolean>) => void;
  };
  let { target, iconOnly = false, onFailure }: Props = $props();
  const { client, navigation } = getAppContext();
  const pin = $derived(navigation.pins.find((pin) => targetsMatch(pin.navigation.target, target)));
  let busy = $state(false);
  let retry = $state.raw<
    { id: number; target: NavigationTarget } | { target: NavigationTarget } | null
  >(null);

  const pending = $derived(retry && targetsMatch(retry.target, target) ? retry : null);

  async function change() {
    let operation = pending ?? (pin ? { id: pin.id, target } : { target });
    function apply() {
      return 'id' in operation
        ? client.unpinTarget(operation.id)
        : client.pinTarget(operation.target);
    }
    busy = true;
    try {
      let saved = await apply();
      if (!saved) onFailure?.(apply);
      retry = saved ? null : operation;
    } catch {
      onFailure?.(apply);
      retry = operation;
    } finally {
      busy = false;
    }
  }
</script>

<div>
  {#if pending}<p role="alert">{m.pins_save_failed()}</p>{/if}
  <Button
    onclick={change}
    loading={busy}
    aria-label={pending ? m.retry() : pin ? m.pins_unpin() : m.pins_pin()}
  >
    {#if iconOnly}<span aria-hidden="true" class={pin ? 'i-mdi-pin' : 'i-mdi-pin-outline'}></span>
    {:else}{pending ? m.retry() : pin ? m.pins_unpin() : m.pins_pin()}{/if}
  </Button>
</div>

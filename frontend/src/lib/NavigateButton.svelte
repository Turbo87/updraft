<script lang="ts">
  import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import { m } from '$lib/paraglide/messages';

  type Props = { target: NavigationTarget; iconOnly?: boolean };
  let { target, iconOnly = false }: Props = $props();
  const { client } = getAppContext();
  let busy = $state(false);
  let error = $state(false);
  async function navigate() {
    busy = true;
    error = false;
    try {
      if (await client.setNavigationTarget(target)) await goto(resolve('/'));
      else error = true;
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
</script>

<Button onclick={navigate} loading={busy} aria-label={m.pins_navigate()}>
  {#if iconOnly}<span aria-hidden="true" class="i-mdi-navigation"
    ></span>{:else}{m.pins_navigate()}{/if}
</Button>
{#if error}<p role="alert">{m.navigation_failed()}</p>{/if}

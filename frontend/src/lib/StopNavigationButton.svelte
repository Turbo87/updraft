<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import { m } from '$lib/paraglide/messages';

  const { client } = getAppContext();
  let error = $state(false);
  let busy = $state(false);
  async function stop() {
    busy = true;
    error = false;
    try {
      let saved = await client.setNavigationTarget(null);
      if (!saved) {
        error = true;
        return;
      }
      await goto(resolve('/'));
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
</script>

{#if error}<p role="alert">{m.navigation_failed()}</p>{/if}
<Button loading={busy} onclick={stop}>{m.navigation_stop()}</Button>

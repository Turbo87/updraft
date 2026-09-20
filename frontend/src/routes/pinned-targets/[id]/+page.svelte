<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import { navigationLabel } from '$lib/navigation';
  import { m } from '$lib/paraglide/messages';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { client, navigation } = getAppContext();
  const id = $derived(Number(page.params.id));
  const pin = $derived(navigation.pins.find((pin) => pin.id === id));
  let error = $state<'navigation' | 'pins' | null>(null);
  let busy = $state(false);
  async function navigate() {
    if (!pin) return;
    busy = true;
    error = null;
    try {
      if (await client.setNavigationTarget(pin.navigation.target)) await goto(resolve('/'));
      else error = 'navigation';
    } catch {
      error = 'navigation';
    } finally {
      busy = false;
    }
  }
  async function unpin() {
    busy = true;
    error = null;
    try {
      if (await client.unpinTarget(id)) await goto(resolve('/'));
      else error = 'pins';
    } catch {
      error = 'pins';
    } finally {
      busy = false;
    }
  }
</script>

<ScreenScaffold
  title={pin ? navigationLabel(pin.navigation) : m.pins_details()}
  backHref="/"
  backLabel={m.flight_view()}
>
  {#if pin?.navigation.position}
    <p>
      {pin.navigation.position.latitudeDegrees.toFixed(5)}°, {pin.navigation.position.longitudeDegrees.toFixed(
        5,
      )}°
    </p>
  {:else}<p>{pin ? m.navigation_waiting() : m.pins_missing()}</p>{/if}
  {#if error}<p role="alert">
      {error === 'pins' ? m.pins_save_failed() : m.navigation_failed()}
    </p>{/if}
  {#snippet actions()}
    {#if pin}<Button onclick={navigate} loading={busy}>{m.pins_navigate()}</Button>{/if}
    {#if pin || error === 'pins'}<Button onclick={unpin} loading={busy}
        >{error === 'pins' ? m.retry() : m.pins_unpin()}</Button
      >{/if}
  {/snippet}
</ScreenScaffold>

<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import { m } from '$lib/paraglide/messages';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { client, navigation } = getAppContext();
  let error = $state(false);
  let busy = $state(false);
  async function stop() {
    busy = true;
    error = false;
    try {
      await client.setNavigationTarget(null);
      await goto(resolve('/'));
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
</script>

<ScreenScaffold title={m.navigation_heading()} backHref="/" backLabel={m.flight_view()}>
  {#if navigation.current}
    <h2>{navigation.current.target.name}</h2>
    <p>
      {navigation.current.position.latitudeDegrees.toFixed(5)}°, {navigation.current.position.longitudeDegrees.toFixed(
        5,
      )}°
    </p>
  {:else}<p>{m.navigation_none()}</p>{/if}
  {#if error}<p role="alert">{m.navigation_failed()}</p>{/if}
  {#snippet actions()}<Button loading={busy} onclick={stop}>{m.navigation_stop()}</Button>{/snippet}
</ScreenScaffold>

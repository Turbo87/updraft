<script lang="ts">
  import { resolve } from '$app/paths';

  import { getAppContext } from '#lib/app-context.js';
  import { navigationLabel } from '#lib/navigation.js';
  import { m } from '#lib/paraglide/messages.js';
  import PinTargetButton from '#lib/PinTargetButton.svelte';
  import ScreenScaffold from '#lib/ScreenScaffold.svelte';
  import StopNavigationButton from '#lib/StopNavigationButton.svelte';

  const { navigation } = getAppContext();
</script>

<ScreenScaffold title={m.navigation_heading()} backHref={resolve('/')} backLabel={m.flight_view()}>
  {#if navigation.current}
    <h2>{navigationLabel(navigation.current)}</h2>
    {#if navigation.current.position}
      <p>
        {navigation.current.position.latitudeDegrees.toFixed(5)}°, {navigation.current.position.longitudeDegrees.toFixed(
          5,
        )}°
      </p>
    {:else}<p>{m.navigation_waiting()}</p>{/if}
  {:else}<p>{m.navigation_none()}</p>{/if}
  {#snippet actions()}<StopNavigationButton />
    {#if navigation.current}<PinTargetButton target={navigation.current.target} />{/if}
  {/snippet}
</ScreenScaffold>

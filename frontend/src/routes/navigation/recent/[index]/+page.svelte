<script lang="ts">
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import NavigateButton from '$lib/NavigateButton.svelte';
  import { navigationLabel } from '$lib/navigation';
  import { m } from '$lib/paraglide/messages';
  import PinTargetButton from '$lib/PinTargetButton.svelte';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { navigation } = getAppContext();
  const target = $state(navigation.recents[Number(page.params.index)]);
</script>

<ScreenScaffold
  title={target ? navigationLabel({ target, traffic: null }) : m.navigation_none()}
  backHref="/navigation"
  backLabel={m.navigation_heading()}
>
  {#if target && target.type !== 'traffic'}<p>
      {target.latitudeDegrees.toFixed(5)}°, {target.longitudeDegrees.toFixed(5)}°
    </p>{/if}
  {#snippet actions()}
    {#if target}<NavigateButton {target} /><PinTargetButton {target} />{/if}
  {/snippet}
</ScreenScaffold>

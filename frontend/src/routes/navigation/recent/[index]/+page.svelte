<script lang="ts">
  import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';

  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import NavigateButton from '$lib/NavigateButton.svelte';
  import { navigationLabel } from '$lib/navigation';
  import { m } from '$lib/paraglide/messages';
  import PinTargetButton from '$lib/PinTargetButton.svelte';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { navigation } = getAppContext();
  let selected = $state.raw<{ index: string | undefined; target: NavigationTarget } | null>(null);
  const index = $derived(page.params.index);
  const target = $derived(selected?.index === index ? selected?.target : null);
  $effect(() => {
    if (selected?.index !== index && navigation.recents[Number(index)]) {
      selected = { index, target: navigation.recents[Number(index)] };
    }
  });
</script>

<ScreenScaffold
  title={target ? navigationLabel({ target, traffic: null }) : m.navigation_none()}
  backHref="/navigation"
  backLabel={m.navigation_heading()}
>
  {#if target && target.type !== 'traffic' && target.type !== 'task'}<p>
      {target.latitudeDegrees.toFixed(5)}°, {target.longitudeDegrees.toFixed(5)}°
    </p>{/if}
  {#snippet actions()}
    {#if target}<NavigateButton {target} /><PinTargetButton {target} />{/if}
  {/snippet}
</ScreenScaffold>

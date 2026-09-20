<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import PinTargetButton from '$lib/PinTargetButton.svelte';
  import TrafficDetails from './TrafficDetails.svelte';

  const { client, instruments, settings, traffic } = getAppContext();
  const trafficId = $derived(page.params.id);
  const locale = $derived(settings.current.locale ?? getLocale());

  let navigationError = $state(false);
  let navigating = $state(false);
  async function navigate() {
    if (!trafficId) return;
    navigating = true;
    navigationError = false;
    try {
      if (await client.setNavigationTarget({ type: 'traffic', id: trafficId })) {
        await goto(resolve('/'));
      } else {
        navigationError = true;
      }
    } catch {
      navigationError = true;
    } finally {
      navigating = false;
    }
  }
  function goBack() {
    history.back();
  }
</script>

<TrafficDetails
  onNavigate={navigate}
  {pinAction}
  {navigationError}
  {navigating}
  backLabel={m.traffic_back()}
  id={trafficId ?? ''}
  {instruments}
  {locale}
  onBack={goBack}
  {traffic}
  units={settings.current.units}
/>

{#snippet pinAction()}{#if trafficId}<PinTargetButton
      target={{ type: 'traffic', id: trafficId }}
    />{/if}{/snippet}

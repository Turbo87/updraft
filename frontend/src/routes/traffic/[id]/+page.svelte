<script lang="ts">
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import TrafficDetails from './TrafficDetails.svelte';

  const { instruments, settings, traffic } = getAppContext();
  const trafficId = $derived(page.params.id);
  const locale = $derived(settings.current.locale ?? getLocale());

  function goBack() {
    history.back();
  }
</script>

<TrafficDetails
  backLabel={m.traffic_back()}
  id={trafficId ?? ''}
  {instruments}
  {locale}
  onBack={goBack}
  {traffic}
  units={settings.current.units}
/>

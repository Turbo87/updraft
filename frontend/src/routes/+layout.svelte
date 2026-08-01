<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import { onMount } from 'svelte';
  import { page } from '$app/state';

  import { setAppContext } from '$lib/app-context';
  import favicon from '$lib/assets/favicon.svg';
  import { FakeClient } from '$lib/client/fake';
  import { TauriClient } from '$lib/client/tauri';
  import FlightView from '$lib/flight-view/FlightView.svelte';
  import { applyLocaleSetting } from '$lib/i18n.svelte.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { InstrumentsStore } from '$lib/stores/instruments.svelte';
  import { SettingsStore } from '$lib/stores/settings.svelte';
  import { TrafficStore } from '$lib/stores/traffic.svelte';

  type TestWindow = Window & { __updraftFake?: FakeClient };

  let { children } = $props();

  const instruments = new InstrumentsStore();
  const settings = new SettingsStore();
  const traffic = new TrafficStore();
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';
  const inTauri = '__TAURI_INTERNALS__' in window;
  const client = inTauri ? new TauriClient() : new FakeClient();

  setAppContext({ client, settings });

  // Only in test mode: a plain web build should not hand every visitor a
  // handle for injecting instrument data.
  if (testMode && client instanceof FakeClient) {
    (window as TestWindow).__updraftFake = client;
  }

  onMount(() => {
    return client.subscribe((topic) => {
      instruments.apply(topic);
      settings.apply(topic);
      traffic.apply(topic);
      if (topic.topic === 'settings') {
        applyLocaleSetting(topic.value.locale);
      }
    });
  });

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<div class="app">
  <FlightView
    instruments={instruments.current}
    {traffic}
    units={settings.current.units}
    {testMode}
  />
  {#if page.url.pathname !== '/'}
    <div class="route-content">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .app,
  .route-content {
    position: absolute;
    inset: 0;
  }
</style>

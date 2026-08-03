<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import type { AppContext } from '$lib/app-context';

  import { onMount } from 'svelte';
  import { page } from '$app/state';

  import { setAppContext } from '$lib/app-context';
  import favicon from '$lib/assets/favicon.svg';
  import { FakeClient } from '$lib/client/fake';
  import { TauriClient } from '$lib/client/tauri';
  import FlightView from '$lib/flight-view/FlightView.svelte';
  import { applyLocaleSetting } from '$lib/i18n.svelte.js';
  import { MapState } from '$lib/map-state.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { AirspaceStore } from '$lib/stores/airspace.svelte';
  import { ExternalDevicesStore } from '$lib/stores/external-devices.svelte';
  import { InstrumentsStore } from '$lib/stores/instruments.svelte';
  import { SettingsStore } from '$lib/stores/settings.svelte';
  import { TrafficStore } from '$lib/stores/traffic.svelte';

  type TestWindow = Window & {
    __updraftApp?: AppContext;
    __updraftFake?: FakeClient;
  };

  let { children } = $props();

  const externalDevices = new ExternalDevicesStore();
  const instruments = new InstrumentsStore();
  const airspace = new AirspaceStore();
  const mapState = new MapState();
  const settings = new SettingsStore();
  const traffic = new TrafficStore();
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';
  const inTauri = '__TAURI_INTERNALS__' in window;
  const client = inTauri ? new TauriClient() : new FakeClient();
  const appContext = { client, airspace, externalDevices, mapState, settings } satisfies AppContext;

  setAppContext(appContext);

  // Only test mode exposes application state and the fake client to browser automation.
  if (testMode) {
    let testWindow = window as TestWindow;
    testWindow.__updraftApp = appContext;
    if (client instanceof FakeClient) testWindow.__updraftFake = client;
  }

  onMount(() => {
    return client.subscribe((topic) => {
      externalDevices.apply(topic);
      instruments.apply(topic);
      airspace.apply(topic);
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
    airspace={airspace.current}
    instruments={instruments.current}
    {mapState}
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

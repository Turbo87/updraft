<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import { onMount } from 'svelte';
  import { page } from '$app/state';

  import favicon from '$lib/assets/favicon.svg';
  import { FakeClient } from '$lib/client/fake';
  import { TauriClient } from '$lib/client/tauri';
  import FlightView from '$lib/flight-view/FlightView.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { InstrumentsStore } from '$lib/stores/instruments.svelte';

  type TestWindow = Window & { __updraftFake?: FakeClient };

  let { children } = $props();

  const instruments = new InstrumentsStore();
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';

  onMount(() => {
    let inTauri = '__TAURI_INTERNALS__' in window;
    let client = inTauri ? new TauriClient() : new FakeClient();

    // Only in test mode: a plain web build should not hand every visitor a
    // handle for injecting instrument data.
    if (testMode && client instanceof FakeClient) {
      (window as TestWindow).__updraftFake = client;
    }

    return client.subscribe((topic) => instruments.apply(topic));
  });

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<div class="app">
  <FlightView instruments={instruments.current} {testMode} />
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

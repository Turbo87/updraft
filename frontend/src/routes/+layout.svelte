<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import { onMount } from 'svelte';
  import { page } from '$app/state';

  import favicon from '$lib/assets/favicon.svg';
  import FlightView from '$lib/flight-view/FlightView.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { HttpUpdraftClient } from '$lib/protocol/client';
  import { ApplicationState } from '$lib/protocol/state.svelte';

  let { children } = $props();

  const state = new ApplicationState();
  const client = new HttpUpdraftClient();
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';

  onMount(() => client.subscribe(state));

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<div class="app">
  <FlightView position={state.flight.position} {testMode} />
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

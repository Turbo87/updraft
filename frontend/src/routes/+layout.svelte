<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import type { Snippet } from 'svelte';
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
  import { BasemapsStore } from '$lib/stores/basemaps.svelte';
  import { DataActivation } from '$lib/stores/data-activation.svelte';
  import { EnrouteCatalogStore } from '$lib/stores/enroute-catalog.svelte';
  import { EnrouteDownloadsStore } from '$lib/stores/enroute-downloads.svelte';
  import { ExternalDevicesStore } from '$lib/stores/external-devices.svelte';
  import { GlidePerformanceStore } from '$lib/stores/glide-performance.svelte';
  import { InstrumentsStore } from '$lib/stores/instruments.svelte';
  import { SettingsStore } from '$lib/stores/settings.svelte';
  import { TerrainStore } from '$lib/stores/terrain.svelte';
  import { TrafficStore } from '$lib/stores/traffic.svelte';
  import { WaypointsStore } from '$lib/stores/waypoints.svelte';

  type TestWindow = Window & {
    __updraftApp?: AppContext;
    __updraftFake?: FakeClient;
  };

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  const externalDevices = new ExternalDevicesStore();
  const instruments = new InstrumentsStore();
  const airspace = new AirspaceStore();
  const basemaps = new BasemapsStore();
  const terrain = new TerrainStore();
  const enrouteCatalog = new EnrouteCatalogStore();
  const enrouteDownloads = new EnrouteDownloadsStore();
  const waypoints = new WaypointsStore();
  const mapState = new MapState();
  const settings = new SettingsStore();
  const glidePerformance = new GlidePerformanceStore();
  const traffic = new TrafficStore();
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';
  const inTauri = '__TAURI_INTERNALS__' in window;
  const client = inTauri ? new TauriClient() : new FakeClient();
  const dataActivation = new DataActivation(client, airspace, waypoints, basemaps, terrain);
  const appContext = {
    client,
    dataActivation,
    airspace,
    basemaps,
    terrain,
    enrouteCatalog,
    enrouteDownloads,
    waypoints,
    externalDevices,
    instruments,
    mapState,
    settings,
    glidePerformance,
    traffic,
  } satisfies AppContext;

  setAppContext(appContext);

  onMount(() => enrouteCatalog.watchUpdates(client, basemaps, terrain));

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
      waypoints.apply(topic);
      dataActivation.apply(topic);
      settings.apply(topic);
      glidePerformance.apply(topic);
      traffic.apply(topic);
      if (topic.topic === 'settings') {
        applyLocaleSetting(topic.value.locale);
      }
    });
  });

  onMount(() => {
    let subscription = client.subscribeBasemaps(
      (status) => {
        basemaps.current = status;
        dataActivation.apply({ topic: 'basemap', value: status });
      },
      () => {
        basemaps.error = true;
      },
    );
    return () => {
      void subscription.close().catch((error: unknown) => {
        console.warn('Could not close basemap subscription', error);
      });
    };
  });

  onMount(() => {
    let subscription = client.subscribeTerrain(
      (status) => {
        terrain.current = status;
        dataActivation.apply({ topic: 'terrain', value: status });
      },
      () => {
        terrain.error = true;
      },
    );
    return () => {
      void subscription.close().catch((error: unknown) => {
        console.warn('Could not close terrain subscription', error);
      });
    };
  });

  onMount(() => {
    let subscription = client.subscribeEnrouteCatalog(
      (status) => {
        enrouteCatalog.current = status;
      },
      () => {
        enrouteCatalog.error = true;
      },
    );
    return () => {
      void subscription.close().catch((error: unknown) => {
        console.warn('Could not close Enroute catalog subscription', error);
      });
    };
  });

  onMount(() => {
    let subscription = client.subscribeEnrouteDownloads(
      (status) => {
        enrouteDownloads.current = status;
      },
      () => {
        enrouteDownloads.error = true;
      },
    );
    return () => {
      void subscription.close().catch((error: unknown) => {
        console.warn('Could not close Enroute download subscription', error);
      });
    };
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
    {client}
    airspace={airspace.current}
    basemapGeneration={basemaps.current?.generation ?? 0}
    terrainGeneration={terrain.current?.generation ?? 0}
    waypoints={waypoints.current}
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

<script lang="ts">
  import { beforeNavigate } from '$app/navigation';

  import { getAppContext } from '$lib/app-context';
  import DataLibrary from '$lib/DataLibrary.svelte';

  const {
    client,
    airspace,
    basemaps,
    terrain,
    waypoints,
    dataActivation,
    enrouteCatalog,
    enrouteDownloads,
  } = getAppContext();
  let library: { handleBack(): boolean };

  beforeNavigate((navigation) => {
    if (navigation.type === 'popstate' && library.handleBack()) navigation.cancel();
  });
</script>

<DataLibrary
  downloads={enrouteDownloads.current}
  downloadError={enrouteDownloads.error}
  onDownload={(paths) => client.downloadEnrouteBasemaps(paths)}
  onCancelDownload={(path) => client.cancelEnrouteDownload(path)}
  catalog={enrouteCatalog.current}
  catalogError={enrouteCatalog.error}
  updateCheckError={enrouteCatalog.basemapUpdateError}
  onRetryCatalog={() => client.refreshEnrouteCatalog()}
  onCheckBasemapUpdates={() => client.getEnrouteBasemapUpdates()}
  basemaps={basemaps.current}
  basemapError={basemaps.error}
  terrain={terrain.current}
  terrainError={terrain.error}
  importer={client}
  activation={dataActivation}
  airspace={airspace.current}
  waypoints={waypoints.current}
  bind:this={library}
  onRemove={(type, name) =>
    type === 'terrain'
      ? client.removeTerrain(name)
      : type === 'basemap'
        ? client.removeBasemap(name)
        : type === 'airspace'
          ? client.removeAirspace(name)
          : client.removeWaypoints(name)}
/>

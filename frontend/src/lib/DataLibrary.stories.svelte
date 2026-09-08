<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import DataLibrary from './DataLibrary.svelte';

  type Args = ComponentProps<typeof DataLibrary>;

  const { Story } = defineMeta({
    title: 'Screens/DataLibrary',
    component: DataLibrary,
    args: {
      catalog: null,
      onRetryCatalog: fn(),
      onCheckBasemapUpdates: fn(async () => []),
      onReadBasemapDetails: fn(async () => ({
        size: 61_000_000,
        modifiedAt: Date.UTC(2026, 8, 8, 12),
      })),
      onDownload: fn(),
      onCancelDownload: fn(),
      onRemove: fn(),
      importer: {
        selectDataFile: fn(async () => null),
        importDataFile: fn(),
        discardDataFile: fn(),
      },
      activation: {
        pending: false,
        isEnabled: (_type, source) => source.type !== 'disabled',
        hasError: () => false,
        setEnabled: fn(),
      },
      airspace: { generation: 0, sources: [] },
      waypoints: { generation: 0, sources: [] },
    },
    parameters: { layout: 'fullscreen' },
  });
</script>

{#snippet template(args: Args)}
  <div class="screen"><DataLibrary {...args} /></div>
{/snippet}

<Story name="Empty" {template} />
<Story
  name="Imported files"
  {template}
  args={{
    airspace: {
      generation: 1,
      sources: [
        { type: 'active', sourceName: 'Germany.txt', airspaceCount: 1200 },
        { type: 'unavailable', sourceName: 'alps_gliding_sectors.txt', error: 'parseFailed' },
        { type: 'disabled', sourceName: 'Local airspace.txt' },
      ],
    },
    waypoints: {
      generation: 1,
      sources: [
        {
          type: 'active',
          sourceName: 'club_outlandings.cup',
          waypointCount: 86,
          warnings: [
            { line: 4, message: 'Skipped waypoint' },
            { line: 5, message: 'Ignored field' },
          ],
        },
        {
          type: 'disabled',
          sourceName:
            'A long imported filename that must wrap without hiding its disabled state.cup',
        },
        { type: 'unavailable', sourceName: 'regional.cup', error: 'readFailed' },
      ],
    },
  }}
/>

<Story
  name="Replacement confirmation"
  {template}
  args={{
    waypoints: { generation: 1, sources: [{ type: 'disabled', sourceName: 'local.cup' }] },
    importer: {
      selectDataFile: fn(async () => ({
        selectionId: '1',
        sourceName: 'local.cup',
        dataType: 'waypoints' as const,
      })),
      importDataFile: fn(),
      discardDataFile: fn(),
    },
  }}
/>

<Story
  name="Basemap files"
  {template}
  args={{
    basemaps: {
      generation: 0,
      sources: [
        { sourceName: 'France.mbtiles', type: 'active' },
        { sourceName: 'Germany.mbtiles', type: 'disabled' },
        { sourceName: 'Damaged regional basemap.mbtiles', type: 'unavailable' },
      ],
    },
  }}
/>
<Story name="Loading basemaps" {template} args={{ basemaps: null }} />
<Story name="Basemap inventory failure" {template} args={{ basemaps: null, basemapError: true }} />

<Story
  name="Terrain files"
  {template}
  args={{
    terrain: {
      generation: 0,
      sources: [
        { sourceName: 'France.terrain', type: 'active' },
        { sourceName: 'Germany.terrain', type: 'disabled' },
        { sourceName: 'Incompatible regional elevation data.terrain', type: 'unavailable' },
      ],
    },
  }}
/>
<Story name="Loading terrain" {template} args={{ terrain: null }} />
<Story name="Terrain inventory failure" {template} args={{ terrain: null, terrainError: true }} />

<Story
  name="Downloads and installed update"
  {template}
  args={{
    basemaps: {
      generation: 1,
      sources: [{ sourceName: 'enroute/Europe/Germany.mbtiles', type: 'disabled' }],
    },
    downloads: [
      {
        path: 'Europe/Germany.mbtiles',
        type: 'downloading',
        downloaded: 50_000_000,
        total: 150_000_000,
      },
      { path: 'Europe/Malta.mbtiles', type: 'queued' },
      { path: 'Europe/France.mbtiles', type: 'failed' },
    ],
  }}
/>

<Story
  name="Update check failed"
  {template}
  args={{
    updateCheckError: true,
    catalog: {
      cached: { entries: [], checkedAt: Date.UTC(2026, 8, 7, 12) },
      refreshing: false,
      error: true,
    },
  }}
/>
<Story
  name="Update check never succeeded"
  {template}
  args={{
    updateCheckError: true,
    catalog: { cached: null, refreshing: false, error: true },
  }}
/>

<Story
  name="Updates available"
  {template}
  args={{
    updates: ['Europe/Germany.mbtiles'],
    catalog: {
      cached: {
        checkedAt: Date.UTC(2026, 8, 8),
        entries: [
          {
            path: 'Europe/Germany.mbtiles',
            countryCode: 'DE',
            continent: 'europe',
            publicationDate: '2026-09-08',
            size: 378_000_000,
          },
        ],
      },
      refreshing: false,
      error: false,
    },
    basemaps: {
      generation: 1,
      sources: [{ sourceName: 'enroute/Europe/Germany.mbtiles', type: 'disabled' }],
    },
  }}
/>

<Story
  name="Download unavailable basemap again"
  {template}
  args={{
    updates: [],
    catalog: {
      cached: {
        checkedAt: Date.UTC(2026, 8, 8),
        entries: [
          {
            path: 'Europe/France.mbtiles',
            countryCode: 'FR',
            continent: 'europe',
            publicationDate: '2026-09-08',
            size: 61_000_000,
          },
        ],
      },
      refreshing: false,
      error: false,
    },
    basemaps: {
      generation: 1,
      sources: [{ sourceName: 'enroute/Europe/France.mbtiles', type: 'unavailable' }],
    },
  }}
/>

<style>
  .screen {
    height: 100dvh;
  }
</style>

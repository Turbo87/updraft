<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import DataCountry from './DataCountry.svelte';

  const paths = ['Europe/France/North.mbtiles', 'Europe/France/South.mbtiles'];
  const { Story } = defineMeta({
    title: 'Screens/DataCountry',
    component: DataCountry,
    args: {
      country: 'FR',
      entries: paths.map((path) => ({
        path,
        countryCode: 'FR',
        continent: 'europe' as const,
        size: 150_000_000,
        publicationDate: '2026-09-08',
      })),
      basemaps: { generation: 0, sources: [] },
      downloads: [],
      client: {
        getEnrouteBasemapUpdates: fn(async () => [paths[1]]),
        downloadEnrouteBasemaps: fn(async () => {}),
        cancelEnrouteDownload: fn(async () => {}),
      },
      onBack: fn(),
      onDownloaded: fn(),
    },
    parameters: { layout: 'fullscreen' },
  });
</script>

{#snippet template(args: ComponentProps<typeof DataCountry>)}
  <div style="height: 100dvh"><DataCountry {...args} /></div>
{/snippet}

<Story name="Available regions" {template} />
<Story
  name="Installed and update"
  {template}
  args={{
    basemaps: {
      generation: 1,
      sources: paths.map((path) => ({ sourceName: `enroute/${path}`, type: 'disabled' })),
    },
  }}
/>
<Story
  name="Downloading and queued"
  {template}
  args={{
    downloads: [
      { path: paths[0], type: 'downloading', downloaded: 50_000_000, total: 150_000_000 },
      { path: paths[1], type: 'queued' },
    ],
  }}
/>
<Story name="Unavailable state" {template} args={{ downloads: null, stateError: true }} />

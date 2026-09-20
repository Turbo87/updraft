<script module lang="ts">
  import type { ComponentProps } from 'svelte';
  import type { EnrouteCatalogStatus } from './client';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import DataCatalog from './DataCatalog.svelte';
  import { catalogEntries } from './enroute-catalog.fixture';

  const cached: NonNullable<EnrouteCatalogStatus['cached']> = {
    checkedAt: Date.UTC(2026, 8, 8, 12),
    entries: catalogEntries(),
  };
  const { Story } = defineMeta({
    title: 'Screens/DataCatalog',
    component: DataCatalog,
    args: {
      status: { cached, refreshing: false, error: false },
      onBack: fn(),
      onCountry: fn(),
      onImport: fn(),
      onRetry: fn(),
    },
    parameters: { layout: 'fullscreen' },
  });
</script>

{#snippet template(args: ComponentProps<typeof DataCatalog>)}
  <div style="height: 100dvh"><DataCatalog {...args} /></div>
{/snippet}

<Story name="Countries" {template} />
<Story name="Loading" {template} args={{ status: null }} />
<Story
  name="Unavailable"
  {template}
  args={{ status: { cached: null, refreshing: false, error: true } }}
/>
<Story
  name="Cached after failure"
  {template}
  args={{ status: { cached, refreshing: false, error: true } }}
/>
<Story name="Retrying" {template} args={{ status: { cached, refreshing: true, error: true } }} />
<Story
  name="Empty"
  {template}
  args={{ status: { cached: { ...cached, entries: [] }, refreshing: false, error: false } }}
/>

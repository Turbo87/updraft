<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { createRawSnippet } from 'svelte';
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ListRow from './ListRow.svelte';

  const connected = createRawSnippet(() => ({
    render: () =>
      '<span style="display:inline-flex;align-items:center;gap:var(--space-2);padding:var(--space-1) var(--space-3);border-radius:1rem;background:var(--color-success-surface);color:var(--color-success-text);font:var(--text-caption);font-weight:600"><span aria-hidden="true" style="width:0.5rem;height:0.5rem;border-radius:50%;background:currentColor"></span>Connected</span>',
  }));

  const { Story } = defineMeta({
    title: 'Components/ListRow',
    component: ListRow,
    parameters: {
      docs: {
        description: {
          component:
            'Use a list row for one label with a value, status, or navigation target. The standard size is at least 48 pixels high. The large size is at least 64 pixels high and is required for navigation. A navigating row uses a chevron as its only navigation affordance. The full row is the target. Use an icon only when it has a consistent meaning across the application. Use the numeric style for measured values. Use trailing content for a read-only status component. A disabled navigating row keeps its large shape but removes the link target and chevron.',
        },
      },
    },
  });

  type Args = ComponentProps<typeof ListRow>;
</script>

{#snippet template(args: Args)}
  <div class="list-row-story">
    <ListRow {...args} />
  </div>
{/snippet}

<Story name="Label and value" args={{ label: 'Language', value: 'English' }} {template} />
<Story
  name="Numeric value"
  args={{ label: 'Altitude', numeric: true, value: '1245 m MSL' }}
  {template}
/>
<Story
  name="Label and status"
  args={{ label: 'TCP · 192.168.4.1:2000', trailing: connected }}
  {template}
/>
<Story
  name="Navigation"
  args={{
    href: '/settings/airspace',
    icon: 'i-mdi-vector-square',
    label: 'Airspace',
    size: 'large',
    value: 'Germany 2026',
  }}
  {template}
/>
<Story
  name="Navigation without value"
  args={{ href: '/settings/about', label: 'About', size: 'large' }}
  {template}
/>
<Story
  name="Disabled navigation"
  args={{
    disabled: true,
    href: '/settings/devices',
    icon: 'i-mdi-bluetooth',
    label: 'Bluetooth SPP',
    size: 'large',
    value: 'Unsupported',
  }}
  {template}
/>

<style>
  .list-row-story {
    width: 32rem;
    max-width: calc(100vw - 2rem);
  }
</style>

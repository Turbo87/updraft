<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import StatusPill from './StatusPill.svelte';

  const { Story } = defineMeta({
    title: 'Components/StatusPill',
    component: StatusPill,
    parameters: {
      docs: {
        description: {
          component:
            'Use a status pill for a short read-only state. It is never an interactive control. The visible label communicates the state without relying on color. Add an icon for an alert or a loading indicator for a changing state. Use tinted normal, caution, success, and danger-subtle tones for connection and settings states. Reserve the solid warning and danger tones for the alarm ladder. The default font size is 1rem. Set `--status-pill-font-size` when a composition needs a different size. Padding, gaps, and indicators scale with it.',
        },
      },
    },
  });
</script>

<Story name="Connected" args={{ label: 'Connected', tone: 'success' }} />
<Story name="Connecting" args={{ label: 'Connecting', loading: true, tone: 'caution' }} />
<Story name="Disabled" args={{ label: 'Disabled' }} />
<Story
  name="Failed"
  args={{ icon: 'i-mdi-alert-circle', label: 'Failed', tone: 'danger-subtle' }}
/>
<Story name="Stale" args={{ label: 'Stale', tone: 'caution' }} />
<Story
  name="Important alarm"
  args={{ icon: 'i-mdi-alert', label: 'Alarm · important', tone: 'warning' }}
/>
<Story
  name="Urgent alarm"
  args={{ icon: 'i-mdi-alert', label: 'Alarm · urgent', tone: 'danger' }}
/>
<Story name="Custom size" asChild>
  <StatusPill --status-pill-font-size="1.5rem" label="Connected" tone="success" />
</Story>

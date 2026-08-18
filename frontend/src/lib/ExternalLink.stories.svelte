<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ExternalLink from './ExternalLink.svelte';

  const { Story } = defineMeta({
    title: 'Components/ExternalLink',
    component: ExternalLink,
    parameters: {
      layout: 'centered',
      docs: {
        description: {
          component:
            'Use an external link for destinations outside Updraft. Browser rendering opens a new tab and prevents opener access to the application page. Tauri rendering delegates the URL to the system browser. The component forwards native anchor attributes and does not provide visual styling or content.',
        },
      },
    },
  });
</script>

<Story name="Plain link" args={{ href: 'https://github.com/Turbo87/updraft' }}>
  GitHub repository
</Story>

<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import IconButton from './IconButton.svelte';

  const { Story } = defineMeta({
    title: 'Components/IconButton',
    component: IconButton,
    args: { icon: 'i-mdi-download', label: 'Update' },
    parameters: {
      docs: {
        description: {
          component:
            'Use a round 48-pixel icon button for a familiar action. Supply an accessible label. Use it inside list rows. It uses the raised control surface. The component forwards native button attributes and defaults to type="button".',
        },
      },
    },
  });
</script>

<Story name="List row" asChild>
  <div class="row">
    <span>Germany</span>
    <IconButton icon="i-mdi-download" label="Update" />
    <IconButton icon="i-mdi-refresh" label="Retry" />
    <IconButton icon="i-mdi-close" label="Cancel" />
  </div>
</Story>
<Story name="Disabled" args={{ disabled: true }} />

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--color-card-surface);
    color: var(--color-text);
  }
</style>

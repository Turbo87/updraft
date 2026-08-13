<script module lang="ts">
  import { createRawSnippet } from 'svelte';
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Button from './Button.svelte';

  const importFile = createRawSnippet(() => ({ render: () => '<span>Import file</span>' }));
  const importing = createRawSnippet(() => ({ render: () => '<span>Importing…</span>' }));
  const cancel = createRawSnippet(() => ({ render: () => '<span>Cancel</span>' }));
  const remove = createRawSnippet(() => ({
    render: () =>
      '<span style="display:inline-flex;align-items:center;gap:var(--space-2)"><span aria-hidden="true" class="i-mdi-delete-outline" style="font-size:1.5rem"></span><span>Remove</span></span>',
  }));

  const { Story } = defineMeta({
    title: 'Components/Button',
    component: Button,
    parameters: {
      docs: {
        description: {
          component:
            'Use a standard 48-pixel button inside settings forms. Use a large 56-pixel button for Flight View actions and primary actions in a fixed action bar. Primary, secondary, and destructive roles communicate the consequence of the action. Disabled buttons retain their role colors at reduced opacity. The component forwards native button attributes and uses `type="button"` by default. Loading disables the button, reports `aria-busy`, and replaces rotation with an opacity pulse when reduced motion is active. Override `--button-surface`, `--button-text`, or `--button-height` only when a documented component variant needs it.',
        },
      },
    },
  });
</script>

<Story name="Primary" args={{ children: importFile }} />
<Story name="Primary large" args={{ children: importFile, size: 'large' }} />
<Story name="Secondary" args={{ children: cancel, variant: 'secondary' }} />
<Story name="Destructive" args={{ children: remove, variant: 'destructive' }} />
<Story name="Loading" args={{ children: importing, loading: true, size: 'large' }} />
<Story name="Primary disabled" args={{ children: importFile, disabled: true, size: 'large' }} />
<Story
  name="Secondary disabled"
  args={{ children: cancel, disabled: true, variant: 'secondary' }}
/>
<Story
  name="Destructive disabled"
  args={{ children: remove, disabled: true, variant: 'destructive' }}
/>

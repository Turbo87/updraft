<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { action } from 'storybook/actions';

  import Button from './Button.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';

  const onCancel = action('onCancel');
  const onConfirm = action('onConfirm');

  const { Story } = defineMeta({
    title: 'Components/ConfirmDialog',
    component: ConfirmDialog,
    parameters: {
      layout: 'centered',
      docs: {
        description: {
          component:
            'Use this dialog to confirm an action with a significant consequence. The safe action receives initial focus. Focus stays inside the dialog and returns to the previous element when the dialog closes. Escape cancels the action. Outside interaction does not close the dialog.',
        },
      },
    },
  });
</script>

<script lang="ts">
  let open = $state(false);
</script>

<Story name="Remove airspace file" asChild>
  <Button variant="destructive" onclick={() => (open = true)}>Remove airspace file</Button>
  <ConfirmDialog
    bind:open
    title="Remove Germany 2026.txt?"
    description="The file is deleted from this device and airspace disappears from the map. You can import it again later."
    cancelLabel="Cancel"
    confirmLabel="Remove"
    {onCancel}
    {onConfirm}
  />
</Story>

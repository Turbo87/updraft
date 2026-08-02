<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ExternalDeviceForm from './ExternalDeviceForm.svelte';

  const { Story } = defineMeta({
    title: 'Components/ExternalDeviceForm',
    component: ExternalDeviceForm,
  });
</script>

<Story name="Add TCP" args={{ onSave: async () => {} }} />

<Story
  name="Edit TCP"
  args={{
    device: { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
    onSave: async () => {},
    onDelete: async () => {},
  }}
/>

<Story
  name="Command error"
  args={{
    onSave: async () => {
      throw new Error('Device command rejected');
    },
  }}
/>

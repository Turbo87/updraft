<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import TextField from './TextField.svelte';

  const { Story } = defineMeta({
    title: 'Components/TextField',
    component: TextField,
    parameters: {
      docs: {
        description: {
          component:
            'Use a text field for one short text or numeric value. Place the visible label above the 48-pixel input. An optional hint explains the expected value. An error replaces that hint and combines text, an icon, a tinted surface, and a strong border so that the state does not rely on color alone. Set `inputmode="numeric"` for whole-number fields to use the numeric keyboard and tabular figures. Native input attributes are forwarded. The generated input ID keeps the label and description associated when no explicit ID is supplied.',
        },
      },
    },
  });

  type Args = ComponentProps<typeof TextField>;
</script>

{#snippet template(args: Args)}
  <div class="text-field-story">
    <TextField {...args} />
  </div>
{/snippet}

<Story
  name="Text with hint"
  args={{
    hint: 'Host name or IP address of the instrument.',
    label: 'Host',
    value: '192.168.4.1',
  }}
  {template}
/>
<Story name="Numeric" args={{ inputmode: 'numeric', label: 'Port', value: '2000' }} {template} />
<Story
  name="Error"
  args={{
    error: 'Enter a whole-number port from 1 to 65535.',
    inputmode: 'numeric',
    label: 'Port',
    value: '70000',
  }}
  {template}
/>
<Story
  name="Disabled"
  args={{
    disabled: true,
    hint: 'Standard SPP UUID. Not editable here.',
    label: 'Service UUID',
    value: '00001101-0000-1000-8000-00805F9B34FB',
  }}
  {template}
/>

<style>
  .text-field-story {
    width: 24rem;
    max-width: calc(100vw - 2rem);
  }
</style>

<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import InlineChoiceGroup from './InlineChoiceGroup.svelte';

  const altitudeOptions = [
    { value: 'm', label: 'm' },
    { value: 'ft', label: 'ft' },
  ] as const;
  const verticalSpeedOptions = [
    { value: 'm/s', label: 'm/s' },
    { value: 'kt', label: 'kt' },
    { value: 'ft/min', label: 'ft/min' },
  ] as const;

  const { Story } = defineMeta({
    title: 'Components/InlineChoiceGroup',
    component: InlineChoiceGroup,
    parameters: {
      layout: 'centered',
      docs: {
        description: {
          component:
            'Use an inline choice group for one decision with two or three short options. Each option has the same width and is a 48-pixel target. The highlighted border indicates the selected option without adding content or changing its dimensions. The group keeps native radio semantics and exposes a visible focus ring. Use `RadioList` when labels need more horizontal space. The component is controlled through `value` and `onChange`.',
        },
      },
    },
  });
</script>

<script lang="ts">
  let altitude = $state<'m' | 'ft'>('m');
  let verticalSpeed = $state<'m/s' | 'kt' | 'ft/min'>('m/s');
</script>

<Story name="Two choices" asChild>
  <InlineChoiceGroup
    name="altitude"
    legend="Altitude"
    options={altitudeOptions}
    value={altitude}
    onChange={(value) => (altitude = value)}
  />
</Story>

<Story name="Three choices" asChild>
  <InlineChoiceGroup
    name="vertical-speed"
    legend="Vertical speed"
    options={verticalSpeedOptions}
    value={verticalSpeed}
    onChange={(value) => (verticalSpeed = value)}
  />
</Story>

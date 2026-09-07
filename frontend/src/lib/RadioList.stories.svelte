<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import RadioList from './RadioList.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';

  const distanceOptions = [
    { value: 'km', label: 'Kilometres · km' },
    { value: 'mi', label: 'Miles · mi' },
    { value: 'nm', label: 'Nautical miles · nm' },
  ] as const;
  const languageOptions = [
    { value: 'en', label: 'English', icon: 'i-circle-flags-lang-en' },
    { value: 'de', label: 'Deutsch', icon: 'i-circle-flags-lang-de' },
  ] as const;
  const deviceOptions = [
    {
      value: '00:11:22:33:44:55',
      label: 'Flight recorder',
      description: '00:11:22:33:44:55',
    },
    { value: 'AA:BB:CC:DD:EE:FF', label: 'AA:BB:CC:DD:EE:FF' },
  ] as const;

  const { Story } = defineMeta({
    title: 'Components/RadioList',
    component: RadioList,
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Use a radio list for one decision with labels that need the width of a list row. The options share one responsive card with hairline separators. At widths up to 34rem the card reaches the screen edges. The legend and error message stay inset. A visible native radio indicates the selected option without changing the label width. Options can include a decorative icon or secondary identifier. Hide the legend only when a surrounding screen heading gives the group the same name. Every label is at least a 48-pixel target. A validation error marks and describes the group without relying on color alone. The component is controlled through `value` and `onChange`.',
        },
      },
    },
  });
</script>

<script lang="ts">
  let distance = $state<'km' | 'mi' | 'nm'>('km');
  let language = $state<'en' | 'de'>('en');
  let device = $state<'00:11:22:33:44:55' | 'AA:BB:CC:DD:EE:FF'>('00:11:22:33:44:55');
</script>

<Story name="Distance" asChild>
  <div class="screen">
    <ScreenScaffold title="Radio lists" backLabel="Back" backHref="/settings">
      <RadioList
        name="distance"
        legend="Distance"
        options={distanceOptions}
        value={distance}
        onChange={(value) => (distance = value)}
      />
    </ScreenScaffold>
  </div>
</Story>

<Story name="With descriptions" asChild>
  <div class="screen">
    <ScreenScaffold title="Radio lists" backLabel="Back" backHref="/settings">
      <RadioList
        name="device"
        legend="Bonded device"
        options={deviceOptions}
        value={device}
        onChange={(value) => (device = value)}
      />
    </ScreenScaffold>
  </div>
</Story>

<Story name="Validation error" asChild>
  <div class="screen">
    <ScreenScaffold title="Radio lists" backLabel="Back" backHref="/settings">
      <RadioList
        name="device-error"
        legend="Bonded device"
        options={deviceOptions}
        value=""
        error="Select a bonded Bluetooth device."
        onChange={() => {}}
      />
    </ScreenScaffold>
  </div>
</Story>

<Story name="With icons" asChild>
  <div class="screen">
    <ScreenScaffold title="Radio lists" backLabel="Back" backHref="/settings">
      <RadioList
        hideLegend
        name="language"
        legend="Language"
        options={languageOptions}
        value={language}
        onChange={(value) => (language = value)}
      />
    </ScreenScaffold>
  </div>
</Story>

<style>
  .screen {
    height: 100vh;
  }
</style>

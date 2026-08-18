<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import TrafficSymbol from './TrafficSymbol.svelte';

  const symbols = [
    { label: 'Glider', trafficType: 'glider' },
    { label: 'Aircraft', trafficType: 'towPlane' },
    { label: 'Helicopter', trafficType: 'helicopter' },
    { label: 'Hang glider', trafficType: 'hangGlider' },
    { label: 'Paraglider', trafficType: 'paraglider' },
    { label: 'Jet', trafficType: 'jetAircraft' },
    { label: 'Balloon', trafficType: 'balloon' },
    { label: 'Airship', trafficType: 'airship' },
    { label: 'Unknown', trafficType: 'unknown' },
  ] as const;

  const { Story } = defineMeta({
    title: 'Components/TrafficSymbol',
    component: TrafficSymbol,
    parameters: {
      layout: 'centered',
      docs: {
        description: {
          component:
            'Use a traffic symbol to illustrate a nearby aircraft with the same canonical SVG shape as the map. The symbol is hidden from assistive technology because the surrounding row owns the accessible text. Tow, drop, and piston aircraft share the aircraft symbol. Traffic types without a dedicated sprite use the unknown symbol. A known track rotates directional traffic clockwise from north. Balloons and targets without a track stay upright. Low, important, and urgent alarms use the map alarm colors. Stale traffic keeps its last symbol at 45% opacity. The symbol is one em square by default. Set `--traffic-symbol-size` on the component or a parent when a composition needs an explicit size.',
        },
      },
    },
  });

  type Args = ComponentProps<typeof TrafficSymbol>;
</script>

{#snippet template(args: Args)}
  <div class="specimen">
    <TrafficSymbol {...args} />
  </div>
{/snippet}

<Story name="Sprite catalogue" asChild>
  <div class="catalogue">
    {#each symbols as symbol (symbol.trafficType)}
      <div>
        <TrafficSymbol trafficType={symbol.trafficType} />
        <span>{symbol.label}</span>
      </div>
    {/each}
  </div>
</Story>

<Story name="Directional glider" args={{ trackDegrees: 241, trafficType: 'glider' }} {template} />
<Story name="Large" asChild>
  <TrafficSymbol --traffic-symbol-size="3rem" trackDegrees={241} trafficType="glider" />
</Story>
<Story
  name="Stale tow plane"
  args={{ stale: true, trackDegrees: 118, trafficType: 'towPlane' }}
  {template}
/>
<Story
  name="Low alarm"
  args={{ alarmLevel: 'low', trackDegrees: 241, trafficType: 'glider' }}
  {template}
/>
<Story
  name="Important alarm"
  args={{
    alarmLevel: 'important',
    trackDegrees: 241,
    trafficType: 'glider',
  }}
  {template}
/>
<Story
  name="Urgent alarm"
  args={{ alarmLevel: 'urgent', trackDegrees: 241, trafficType: 'glider' }}
  {template}
/>

<style>
  .specimen {
    --traffic-symbol-size: 2rem;
  }

  .catalogue {
    --traffic-symbol-size: 2rem;

    display: grid;
    grid-template-columns: repeat(3, minmax(6rem, 1fr));
    gap: var(--space-6);
  }

  .catalogue > div {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font: var(--text-caption);
  }
</style>

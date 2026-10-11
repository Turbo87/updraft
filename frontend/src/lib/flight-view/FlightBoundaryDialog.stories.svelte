<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { action } from 'storybook/actions';

  import FlightBoundaryDialog from './FlightBoundaryDialog.svelte';

  const { Story } = defineMeta({
    title: 'Flight View/Flight boundary question',
    component: FlightBoundaryDialog,
    args: {
      question: { cause: 'positionJump', meters: 120_400 },
      distanceUnit: 'km',
      client: {
        startNewFlight: async () => action('startNewFlight')(),
        continueFlight: async () => action('continueFlight')(),
      },
    },
  });
</script>

<Story name="Position jump" />
<Story name="Position jump in nautical miles" args={{ distanceUnit: 'nm' }} />
<Story
  name="GPS time jumped forward"
  args={{ question: { cause: 'fixUtcJump', milliseconds: 5 * 60 * 60 * 1000 } }}
/>
<Story
  name="GPS time jumped back"
  args={{ question: { cause: 'fixUtcJump', milliseconds: -3 * 24 * 60 * 60 * 1000 } }}
/>

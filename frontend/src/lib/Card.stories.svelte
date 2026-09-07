<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Card from './Card.svelte';
  import ResponsiveCard from './ResponsiveCard.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';

  const { Story } = defineMeta({
    title: 'Components/Card',
    component: Card,
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Card owns the surface, shadow, corners, and optional error outline. Content owns padding and separators. ResponsiveCard is shorthand for Card responsive. At widths up to 34rem it extends through the scaffold insets and has square corners. Rows use --card-safe-area-start and --card-safe-area-end to protect their contents.',
        },
      },
    },
  });
</script>

{#snippet rows()}
  <div class="row">First row</div>
  <div class="row">Second row</div>
{/snippet}

<Story name="Default" asChild>
  <div class="screen">
    <ScreenScaffold title="Cards" backLabel="Back" backHref="/settings">
      <Card children={rows} />
    </ScreenScaffold>
  </div>
</Story>
<Story name="Responsive" asChild>
  <div class="screen">
    <ScreenScaffold title="Cards" backLabel="Back" backHref="/settings">
      <ResponsiveCard children={rows} />
    </ScreenScaffold>
  </div>
</Story>
<Story name="Error" asChild>
  <div class="screen">
    <ScreenScaffold title="Cards" backLabel="Back" backHref="/settings">
      <Card children={rows} error />
    </ScreenScaffold>
  </div>
</Story>

<style>
  .screen {
    height: 100vh;
  }
  .row {
    padding: var(--space-4) calc(var(--space-5) + var(--card-safe-area-end)) var(--space-4)
      calc(var(--space-5) + var(--card-safe-area-start));
  }
  .row + .row {
    border-block-start: 1px solid var(--color-separator);
  }
</style>

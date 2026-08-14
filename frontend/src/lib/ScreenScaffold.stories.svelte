<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Button from './Button.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';

  function goBack() {}

  const loremIpsum = [
    'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer posuere erat a ante venenatis dapibus posuere velit aliquet.',
    'Donec ullamcorper nulla non metus auctor fringilla. Vestibulum id ligula porta felis euismod semper.',
    'Aenean lacinia bibendum nulla sed consectetur. Maecenas faucibus mollis interdum. Cras mattis consectetur purus sit amet fermentum.',
    'Praesent commodo cursus magna, vel scelerisque nisl consectetur et. Nulla vitae elit libero, a pharetra augue.',
    'Curabitur blandit tempus porttitor. Etiam porta sem malesuada magna mollis euismod. Morbi leo risus, porta ac consectetur ac, vestibulum at eros.',
    'Sed posuere consectetur est at lobortis. Duis mollis, est non commodo luctus, nisi erat porttitor ligula, eget lacinia odio sem nec elit.',
    'Nullam quis risus eget urna mollis ornare vel eu leo. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus.',
    'Vivamus sagittis lacus vel augue laoreet rutrum faucibus dolor auctor. Fusce dapibus, tellus ac cursus commodo, tortor mauris condimentum nibh.',
    'Cras justo odio, dapibus ac facilisis in, egestas eget quam. Integer posuere erat a ante venenatis dapibus posuere velit aliquet.',
    'Maecenas sed diam eget risus varius blandit sit amet non magna. Donec id elit non mi porta gravida at eget metus.',
  ];

  const { Story } = defineMeta({
    title: 'Components/ScreenScaffold',
    component: ScreenScaffold,
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Use this scaffold for every screen outside the Flight View. The fixed header contains only a return control and the screen title. Use a destination link when the screen has a fixed parent route. Use the callback mode when the correct destination depends on navigation history. The content region is the only scrolling region. Add an action bar only when the screen has a primary action such as import, add, or save. The header and action bar include safe-area insets. Forms and prose have a maximum width of 34rem.',
        },
      },
    },
  });
</script>

{#snippet content()}
  <p>
    Scrolling content belongs in this region. The header and optional action bar stay visible when
    the content is longer than the available screen height.
  </p>
  <p>
    The content uses the screen gutter and stops growing at the documented measure for forms and
    prose.
  </p>
{/snippet}

{#snippet actions()}
  <Button size="large" style="width: 100%">Save changes</Button>
{/snippet}

<Story
  name="Link return"
  args={{ backHref: '/settings', backLabel: 'Back to settings', children: content, title: 'Units' }}
/>
<Story
  name="Callback return"
  args={{ backLabel: 'Back', children: content, onBack: goBack, title: 'Traffic details' }}
/>
<Story
  name="With action bar"
  args={{
    actions,
    backHref: '/settings',
    backLabel: 'Back to settings',
    children: content,
    title: 'Units',
  }}
/>
<Story name="Scrolling content" asChild>
  <div class="scrolling-story">
    <ScreenScaffold {actions} backHref="/settings" backLabel="Back to settings" title="Lorem ipsum">
      {#each loremIpsum as paragraph (paragraph)}
        <p>{paragraph}</p>
      {/each}
    </ScreenScaffold>
  </div>
</Story>

<style>
  .scrolling-story {
    height: 32rem;
  }
</style>

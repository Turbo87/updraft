<script lang="ts">
  import type { ComponentProps } from 'svelte';

  import Card from './Card.svelte';

  type Props = Omit<ComponentProps<typeof Card>, 'responsive'>;

  let props: Props = $props();
</script>

<Card {...props} responsive />

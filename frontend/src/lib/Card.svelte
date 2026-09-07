<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = Omit<HTMLAttributes<HTMLDivElement>, 'children'> & {
    children: Snippet;
    responsive?: boolean;
    error?: boolean;
  };

  let {
    children,
    responsive = false,
    error = false,
    class: className,
    ...attributes
  }: Props = $props();
</script>

<div {...attributes} class={['card', { responsive, error }, className]}>
  {@render children()}
</div>

<style>
  .card {
    --card-safe-area-start: 0px;
    --card-safe-area-end: 0px;
    min-width: 0;
    overflow: hidden;
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    box-shadow: var(--shadow-card);
  }

  .error {
    outline: 2px solid var(--color-action-destructive-surface);
  }

  @media (max-width: 34rem) {
    .responsive {
      --card-safe-area-start: var(--safe-area-left);
      --card-safe-area-end: var(--safe-area-right);
      margin-inline: calc(-1 * var(--content-inset-start, 0px))
        calc(-1 * var(--content-inset-end, 0px));
      border-radius: 0;
    }
  }
</style>

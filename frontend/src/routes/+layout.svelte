<script lang="ts">
  import favicon from '$lib/assets/favicon.svg';
  import { getLocale } from '$lib/paraglide/runtime.js';

  let { children } = $props();

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
  }
</style>

<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';
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

<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Pathname } from '$app/types';
  import { resolve } from '$app/paths';

  // A navigation row is just a link. Drilling down = following an <a href>.
  let { href, value, children }: { href: Pathname; value?: string; children: Snippet } = $props();
</script>

<a class="row" href={resolve(href)}>
  <span class="label">{@render children()}</span>
  {#if value}<span class="value">{value}</span>{/if}
  <span class="chev" aria-hidden="true">&rsaquo;</span>
</a>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 3rem; /* ~48px touch target */
    padding: 0.7rem 0.75rem;
    border-radius: 0.5rem;
    background: #f5f5f5;
    color: inherit;
    text-decoration: none;
  }

  .row:hover {
    background: #ececec;
  }

  .label {
    flex: 1;
    min-width: 0;
  }

  .value {
    color: #595959;
    font-size: 0.9rem;
  }

  .chev {
    color: #767676;
    font-size: 1.2rem;
    line-height: 1;
  }

  @media (prefers-color-scheme: dark) {
    .row {
      background: #2a2a2a;
    }
    .row:hover {
      background: #333;
    }
    .value {
      color: #b0b0b0;
    }
    .chev {
      color: #9a9a9a;
    }
  }
</style>

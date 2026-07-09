<script lang="ts">
  import type { PageData } from './$types';
  import { m } from '$lib/paraglide/messages.js';
  import Row from '$lib/dialog/Row.svelte';

  let { data }: { data: PageData } = $props();

  // Feature names come from the core (mock) and aren't translated, but the
  // category label is frontend chrome.
  const kindLabel = {
    Airspace: m.feature_kind_airspace,
    Airfield: m.feature_kind_airfield,
    Waypoint: m.feature_kind_waypoint,
    Traffic: m.feature_kind_traffic,
  };
</script>

{#if data.features.length}
  <ul class="list">
    {#each data.features as f (f.id)}
      <li>
        <Row href={`/whats-here/${data.coord}/${f.id}`} value={kindLabel[f.kind]()}>{f.name}</Row>
      </li>
    {/each}
  </ul>
{:else}
  <p class="empty">{m.whats_here_empty()}</p>
{/if}

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .empty {
    margin: 0;
    color: #595959;
  }
  @media (prefers-color-scheme: dark) {
    .empty {
      color: #b0b0b0;
    }
  }
</style>

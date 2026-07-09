<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
</script>

<p class="summary">{data.feature.summary}</p>
<dl class="detail">
  {#each Object.entries(data.feature.detail) as [key, value] (key)}
    <dt>{key}</dt>
    <dd>{value}</dd>
  {/each}
</dl>

<style>
  .summary {
    margin: 0 0 0.75rem;
    color: #595959;
  }
  .detail {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.35rem 1rem;
    margin: 0;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin: 0;
  }
  @media (prefers-color-scheme: dark) {
    .summary {
      color: #b0b0b0;
    }
  }
</style>

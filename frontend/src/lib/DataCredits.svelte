<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { parseMapAttributions } from './map-attribution';

  type Props = { attributions: string[] };

  let { attributions }: Props = $props();

  const parsedAttributions = $derived(parseMapAttributions(attributions));
</script>

{#if parsedAttributions.length > 0}
  <section>
    <h2>{m.about_data_credits_heading()}</h2>
    <ul>
      {#each parsedAttributions as attribution (attribution.source)}
        <li>
          {#each attribution.parts as part (part)}
            {#if part.href}
              <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- Attribution URLs are validated external HTTP(S) URLs. -->
              <a href={part.href}>{part.text}</a>
            {:else}
              {part.text}
            {/if}
          {/each}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  section {
    margin-block-start: 2rem;
  }

  h2 {
    margin-block-end: 0.75rem;
  }

  ul {
    display: grid;
    margin: 0;
    padding-inline-start: 1.25rem;
    gap: 0.5rem;
  }
</style>

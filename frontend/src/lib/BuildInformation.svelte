<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';

  let { commitSha, timestamp, locale }: Props = $props();

  const abbreviatedCommitSha = $derived(commitSha?.slice(0, 7));
  const commitUrl = $derived(
    commitSha === undefined ? undefined : `https://github.com/Turbo87/updraft/commit/${commitSha}`,
  );
  const formattedTimestamp = $derived(
    new Intl.DateTimeFormat(locale, {
      dateStyle: 'medium',
      timeStyle: 'medium',
    }).format(new Date(timestamp)),
  );

  interface Props {
    commitSha: string | undefined;
    timestamp: string;
    locale: string;
  }
</script>

<section>
  <h2>{m.about_build_heading()}</h2>
  <dl>
    <dt>{m.about_build_commit()}</dt>
    <dd>
      {#if commitUrl !== undefined && abbreviatedCommitSha !== undefined}
        <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- The commit URL is an external GitHub URL. -->
        <a href={commitUrl}>{abbreviatedCommitSha}</a>
      {:else}
        {m.about_unknown_version()}
      {/if}
    </dd>
    <dt>{m.about_build_time()}</dt>
    <dd><time datetime={timestamp}>{formattedTimestamp}</time></dd>
  </dl>
</section>

<style>
  h2 {
    margin-block-end: 0.75rem;
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    margin: 0;
    gap: 0.5rem 1rem;
  }

  dt {
    color: light-dark(var(--color-gray-600), var(--color-gray-300));
  }

  dd {
    margin: 0;
  }
</style>

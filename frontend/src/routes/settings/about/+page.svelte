<script lang="ts">
  import { resolve } from '$app/paths';

  import BuildInformation from '$lib/BuildInformation.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
</script>

<main>
  <a class="back-link" href={resolve('/settings')}>{m.back_to_settings()}</a>
  <h1>{m.about_heading()}</h1>

  <section>
    <h2>{m.about_source_heading()}</h2>
    <a href="https://github.com/Turbo87/updraft">{m.about_repository_link()}</a>
  </section>

  <BuildInformation
    commitSha={__BUILD_COMMIT_SHA__}
    timestamp={__BUILD_TIMESTAMP__}
    locale={getLocale()}
  />
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  .back-link {
    display: inline-block;
    margin-block-end: 1rem;
  }

  h1 {
    margin-block-start: 0;
  }

  section {
    margin-block-end: 2rem;
  }

  h2 {
    margin-block-end: 0.75rem;
  }
</style>

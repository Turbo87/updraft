<script lang="ts">
  import type { AttributionPart } from './map-attribution';

  import { m } from '$lib/paraglide/messages.js';
  import ExternalLink from './ExternalLink.svelte';
  import { parseMapAttributions } from './map-attribution';
  import ResponsiveCard from './ResponsiveCard.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';

  type AttributionLink = AttributionPart & { href: string };

  type Props = {
    attributions: string[];
    commitSha?: string;
    locale: string;
    timestamp: string;
  };

  let { attributions, commitSha, locale, timestamp }: Props = $props();

  const abbreviatedCommitSha = $derived(commitSha?.slice(0, 7));
  const commitUrl = $derived(
    commitSha === undefined ? undefined : `https://github.com/Turbo87/updraft/commit/${commitSha}`,
  );
  const formattedTimestamp = $derived(
    new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'medium' }).format(
      new Date(timestamp),
    ),
  );
  const parsedAttributions = $derived(parseMapAttributions(attributions));

  function attributionText(parts: AttributionPart[]): string {
    return parts.map(({ text }) => text).join('');
  }

  function attributionLinks(parts: AttributionPart[]): AttributionLink[] {
    return parts.filter(
      (part, index): part is AttributionLink =>
        part.href !== undefined &&
        parts.findIndex(({ href }) => href !== undefined && href === part.href) === index,
    );
  }
</script>

<ScreenScaffold backHref="/settings" backLabel={m.back_to_settings()} title={m.about_heading()}>
  <div class="identity">
    <span aria-hidden="true" class="i-mdi-weather-windy"></span>
    <span>
      <strong>Updraft</strong>
      <span>{m.about_description()}</span>
    </span>
  </div>

  <section>
    <h2>{m.about_build_heading()}</h2>
    <ResponsiveCard>
      <dl>
        <div class="row">
          <dt>{m.about_build_commit()}</dt>
          <dd>
            {#if commitUrl && abbreviatedCommitSha}
              <ExternalLink class="external-link" href={commitUrl}>
                {abbreviatedCommitSha}
              </ExternalLink>
            {:else}
              <span class="muted">{m.about_unknown_version()}</span>
            {/if}
          </dd>
        </div>
        <div class="row">
          <dt>{m.about_build_time()}</dt>
          <dd><time datetime={timestamp}>{formattedTimestamp}</time></dd>
        </div>
      </dl>
      <ExternalLink class="external-link link-row" href="https://github.com/Turbo87/updraft">
        <span aria-hidden="true" class="i-mdi-github leading-icon"></span>
        <span>{m.about_repository_link()}</span>
        <span aria-hidden="true" class="i-mdi-open-in-new external-icon"></span>
      </ExternalLink>
    </ResponsiveCard>
  </section>

  {#if parsedAttributions.length > 0}
    <section>
      <h2>{m.about_data_credits_heading()}</h2>
      <ResponsiveCard>
        <div class="credits">
          {#each parsedAttributions as attribution (attribution.source)}
            <p>{attributionText(attribution.parts)}</p>
            {#each attributionLinks(attribution.parts) as link (link.href)}
              <ExternalLink class="external-link link-row" href={link.href}>
                <span>{link.text}</span>
                <span aria-hidden="true" class="i-mdi-open-in-new external-icon"></span>
              </ExternalLink>
            {/each}
          {/each}
        </div>
      </ResponsiveCard>
    </section>
  {/if}

  <section>
    <h2>{m.about_licences_heading()}</h2>
    <ResponsiveCard>
      <p class="licence">{m.about_licence_description()}</p>
    </ResponsiveCard>
  </section>
</ScreenScaffold>

<style>
  .identity {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin: 0 var(--space-1) var(--space-5);
  }

  .identity > :first-child {
    flex: 0 0 auto;
    color: var(--color-value-accent);
    font-size: 2rem;
  }

  .identity strong,
  .identity strong + span {
    display: block;
  }

  .identity strong {
    font: 700 1.375rem / 1.15 var(--font-ui);
  }

  .identity strong + span {
    color: var(--color-text-muted);
    font: 400 1rem / 1.3 var(--font-ui);
  }

  section + section {
    margin-block-start: var(--space-6);
  }

  h2 {
    margin: 0 var(--space-1) var(--space-2);
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  dl {
    margin: 0;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    min-height: var(--target-min);
    padding-block: var(--space-2);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
  }

  .row + .row,
  section :global(.link-row),
  .credits > :not(:first-child) {
    border-block-start: 1px solid var(--color-separator);
  }

  dt {
    font: var(--text-row-label);
  }

  dd {
    min-width: 0;
    margin: 0;
    font: var(--text-row-detail);
    font-family: var(--font-numeric);
    font-variant-numeric: tabular-nums;
    text-align: end;
  }

  .muted,
  time {
    color: var(--color-text-muted);
  }

  section :global(.external-link) {
    color: var(--color-link);
  }

  section :global(.link-row) {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: var(--target-min);
    padding-block: var(--space-2);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-4) + var(--card-safe-area-end));
    font: 600 1.0625rem / 1.3 var(--font-ui);
    text-decoration: none;
  }

  section :global(.link-row:focus-visible) {
    outline-offset: -2px;
  }

  .leading-icon {
    flex: 0 0 auto;
    font-size: 1.5rem;
  }

  .external-icon {
    flex: 0 0 auto;
    margin-inline-start: auto;
    font-size: 1.25rem;
  }

  .credits p,
  .licence {
    padding-block: var(--space-3);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
    color: var(--color-text);
    font: var(--text-body);
  }

  .credits p,
  .licence {
    margin: 0;
  }
</style>

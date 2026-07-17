<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale, locales, type Locale } from '$lib/paraglide/runtime.js';
  import { setLocale } from '$lib/i18n.svelte.js';

  const localeOptions = {
    en: { label: 'English', icon: 'i-circle-flags-lang-en' },
    de: { label: 'Deutsch', icon: 'i-circle-flags-lang-de' },
  } satisfies Record<Locale, { label: string; icon: string }>;
</script>

<nav aria-label={m.language_label()}>
  <span>{m.language_label()}:</span>
  {#each locales as locale (locale)}
    <button
      type="button"
      aria-label={localeOptions[locale].label}
      aria-pressed={locale === getLocale()}
      onclick={() => setLocale(locale)}
    >
      <span class={['flag', localeOptions[locale].icon]} aria-hidden="true"></span>
    </button>
  {/each}
</nav>

<style>
  nav {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border-radius: 0.5rem;
    background-color: var(--color-overlay-surface);
    color: var(--color-overlay-text);
  }

  button {
    display: grid;
    width: 3rem;
    height: 3rem;
    padding: 0.375rem;
    place-items: center;
    border: 0.125rem solid transparent;
    border-radius: 50%;
    background-color: transparent;
    cursor: pointer;
  }

  button:hover {
    background-color: rgb(255 255 255 / 15%);
  }

  button[aria-pressed='true'] {
    border-color: var(--color-overlay-text);
  }

  .flag {
    font-size: 2rem;
  }
</style>

<script lang="ts">
  import type { Locale } from '$lib/protocol/generated/Locale';

  import { m } from '$lib/paraglide/messages.js';

  type Props = {
    locale: Locale;
    onLocaleChange: (locale: Locale) => void;
  };

  let { locale, onLocaleChange }: Props = $props();

  const options = [
    { locale: 'en', label: 'English', icon: 'i-circle-flags-lang-en' },
    { locale: 'de', label: 'Deutsch', icon: 'i-circle-flags-lang-de' },
  ] satisfies Array<{ locale: Locale; label: string; icon: string }>;
</script>

<fieldset>
  <legend>{m.language_label()}</legend>
  <div class="choices">
    {#each options as option (option.locale)}
      <label>
        <input
          type="radio"
          name="locale"
          value={option.locale}
          checked={locale === option.locale}
          onchange={() => onLocaleChange(option.locale)}
        />
        <span class={['flag', option.icon]} aria-hidden="true"></span>
        <span>{option.label}</span>
      </label>
    {/each}
  </div>
</fieldset>

<style>
  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-block-end: 0.75rem;
    font-weight: 600;
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  label {
    display: flex;
    min-height: 3rem;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  input {
    width: 1rem;
    height: 1rem;
    margin: 0;
    accent-color: var(--color-link);
  }

  .flag {
    flex: none;
    font-size: 2rem;
  }
</style>

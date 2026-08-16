<script lang="ts">
  import type { Locale } from '$lib/protocol/generated/Locale';

  import { languageOptions } from '$lib/language-options';
  import { m } from '$lib/paraglide/messages.js';
  import RadioList from '$lib/RadioList.svelte';

  type Props = {
    locale: Locale;
    onLocaleChange: (locale: Locale) => void;
  };

  let { locale, onLocaleChange }: Props = $props();

  const options = languageOptions.map(({ locale: value, ...option }) => ({ value, ...option }));
</script>

<RadioList
  hideLegend
  name="locale"
  legend={m.language_label()}
  {options}
  value={locale}
  onChange={onLocaleChange}
/>

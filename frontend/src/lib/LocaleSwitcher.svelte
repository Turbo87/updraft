<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';
</script>

<nav aria-label={m.language_label()}>
	{#each locales as locale (locale)}
		<button type="button" disabled={locale === getLocale()} onclick={() => setLocale(locale)}>
			{locale.toUpperCase()}
		</button>
	{/each}
</nav>

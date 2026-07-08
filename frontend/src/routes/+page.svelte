<script lang="ts">
	import Brand from '$lib/Brand.svelte';
	import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
</script>

<main>
	<Brand />
	<LocaleSwitcher />
</main>

<style>
	main {
		font-family: system-ui, sans-serif;
		text-align: center;
		margin-top: 4rem;
	}
</style>

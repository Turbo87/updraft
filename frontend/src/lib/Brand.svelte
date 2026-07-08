<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		tagline?: string;
	};

	let { tagline = m.brand_tagline() }: Props = $props();
</script>

<h1>Updraft</h1>
<p>{tagline}</p>

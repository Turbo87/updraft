<script lang="ts">
	type Props = {
		tagline?: string;
	};

	let { tagline = 'Soaring flight computer' }: Props = $props();
</script>

<h1>Updraft</h1>
<p>{tagline}</p>

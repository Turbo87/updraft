<script lang="ts">
	import 'maplibre-gl/dist/maplibre-gl.css';
	import type { Map } from 'maplibre-gl';
	import { MapLibre } from 'svelte-maplibre-gl';

	// Fixed placeholder position (EDKA Aachen-Merzbrück) until core state drives
	// the map in the `map-position` step.
	const center: [number, number] = [6.186, 50.823];

	let map: Map | undefined = $state();

	function loadSprites() {
		map?.addSprite('updraft-sdf', `${window.location.origin}/sprites/updraft-sdf`);
	}
</script>

<MapLibre
	inlineStyle="height: 100%; width: 100%"
	style="https://tiles.openfreemap.org/styles/positron"
	autoloadGlobalCss={false}
	bind:map
	onload={loadSprites}
	{center}
	zoom={11}
/>

<script lang="ts">
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { MapLibre } from 'svelte-maplibre-gl';

	// Fixed placeholder position (EDKA Aachen-Merzbrück) until core state drives
	// the map in the `map-position` step.
	const center: [number, number] = [6.186, 50.823];
</script>

<MapLibre
	inlineStyle="height: 100%; width: 100%"
	style="https://tiles.openfreemap.org/styles/positron"
	autoloadGlobalCss={false}
	{center}
	zoom={11}
/>

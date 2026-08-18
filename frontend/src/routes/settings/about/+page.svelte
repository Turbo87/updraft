<script lang="ts">
  import AboutScreen from '$lib/AboutScreen.svelte';
  import { getAppContext } from '$lib/app-context';
  import { collectMapSourceAttributions } from '$lib/map-attribution';
  import { getLocale } from '$lib/paraglide/runtime.js';

  const { mapState } = getAppContext();
  const attributions = collectMapSourceAttributions(mapState.map);
</script>

<AboutScreen
  {attributions}
  commitSha={__BUILD_COMMIT_SHA__}
  locale={getLocale()}
  timestamp={__BUILD_TIMESTAMP__}
/>

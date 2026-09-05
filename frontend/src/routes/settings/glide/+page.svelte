<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import GlideSettings from '$lib/GlideSettings.svelte';

  const { client, settings } = getAppContext();
</script>

<GlideSettings
  polar={settings.current.polar}
  getPolars={() => client.getPolars()}
  setPolar={(polar) => client.setPolar(polar)}
/>

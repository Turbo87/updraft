<script lang="ts">
  import { getAppContext } from '#lib/app-context.js';
  import GlideSettings from '#lib/GlideSettings.svelte';

  const { client, settings } = getAppContext();
</script>

<GlideSettings
  polar={settings.current.polar}
  arrivalReserve={settings.current.arrivalReserve}
  altitudeUnit={settings.current.units.altitude}
  setArrivalReserve={(reserve) => client.changeSetting({ type: 'arrivalReserve', reserve })}
  getPolars={() => client.getPolars()}
  setPolar={(polar) => client.setPolar(polar)}
/>

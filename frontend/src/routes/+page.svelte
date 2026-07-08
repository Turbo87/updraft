<script lang="ts">
  import Map from '$lib/map/Map.svelte';
  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
</script>

<div class="map">
  <Map />
  <div class="overlay">
    <LocaleSwitcher />
  </div>
</div>

<style>
  .map {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .overlay {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
  }
</style>

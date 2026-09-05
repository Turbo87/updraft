<script lang="ts">
  import runway from '../../../libs/updraft_sprites/sprites/runway.svg?url';
  import { waypointSymbols } from './waypoint-symbols';

  let { kind, runwayDirection }: { kind: number; runwayDirection?: number } = $props();
  let symbol = $derived(waypointSymbols[kind] ?? waypointSymbols[0]);
  let landable = $derived([2, 3, 4, 5].includes(kind));
</script>

<span class="waypoint-symbol" class:landable aria-hidden="true">
  <span class="shape" style:mask-image={`url("${symbol.image}")`}></span>
  {#if landable && runwayDirection !== undefined}
    <span
      class="runway"
      style:mask-image={`url("${runway}")`}
      style:transform={`rotate(${runwayDirection}deg)`}
    ></span>
  {/if}
</span>

<style>
  .waypoint-symbol {
    position: relative;
    display: block;
    flex: 0 0 auto;
    inline-size: var(--waypoint-symbol-size, 1em);
    block-size: var(--waypoint-symbol-size, 1em);
    color: var(--color-text);
  }
  .landable {
    color: var(--color-violet-700);
  }
  .shape,
  .runway {
    position: absolute;
    inset: 0;
    background: currentcolor;
    mask-position: center;
    mask-size: contain;
    mask-repeat: no-repeat;
  }
  .runway {
    inset: 4.76% 0;
    background: white;
  }
</style>

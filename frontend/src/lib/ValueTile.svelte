<script lang="ts">
  type Props = {
    label: string;
    value: string;
    unit?: string;
    stale?: boolean;
    class?: string;
  };

  let { label, value, unit, stale = false, class: className }: Props = $props();

  let fractionalUnit = $derived(unit?.includes('/') ? unit.split('/', 2) : undefined);
</script>

<div class={['value-tile', className]}>
  <span class="label">{label}</span>
  <span class={['readout', { stale }]}>
    <span class="value">{value}</span>
    {#if fractionalUnit}
      <span aria-label={unit} class="stacked-unit">
        <span>{fractionalUnit[0]}</span>
        <span class="fraction-rule"></span>
        <span>{fractionalUnit[1]}</span>
      </span>
    {:else if unit === '°'}
      <span class="degree-unit">°</span>
    {:else if unit}
      <span class="unit">{unit}</span>
    {/if}
  </span>
</div>

<style>
  .value-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    background: var(--value-tile-surface, var(--color-card-surface));
  }

  .label {
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .readout {
    display: flex;
    align-items: baseline;
    gap: 0.1875rem;
  }

  .value {
    color: var(--color-value-text);
    font: var(--value-tile-value-font, var(--text-value-md));
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }

  .readout.stale .value,
  .readout.stale .unit,
  .readout.stale .degree-unit,
  .readout.stale .stacked-unit {
    color: var(--color-value-stale);
  }

  .unit,
  .degree-unit,
  .stacked-unit {
    color: var(--color-text-muted);
    font-family: var(--font-ui);
    font-weight: 600;
  }

  .unit {
    font-size: var(--value-tile-unit-size, 1rem);
  }

  .degree-unit {
    align-self: flex-start;
    font-size: var(--value-tile-degree-size, 1.125rem);
  }

  .stacked-unit {
    display: flex;
    flex-direction: column;
    align-items: center;
    align-self: center;
    font-size: var(--value-tile-stacked-unit-size, 0.75rem);
    line-height: 1.1;
  }

  .fraction-rule {
    width: 0.75rem;
    height: 1px;
    margin-block: 1px;
    background: currentcolor;
  }
</style>

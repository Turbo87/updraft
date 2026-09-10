<script lang="ts">
  import type { InfoboxValue } from './value';

  import { m } from '$lib/paraglide/messages';
  import { getLocale } from '$lib/paraglide/runtime';
  import { formatInfoboxValue } from './value';

  type Props = {
    label: string;
    value: InfoboxValue;
    stale: boolean;
  };

  let { label, value, stale }: Props = $props();
  const presentation = $derived(formatInfoboxValue(value, getLocale()));
  const stackedUnit = $derived(
    presentation.unit?.includes('/') ? presentation.unit.split('/', 2) : null,
  );
</script>

<div class="infobox" role="group" aria-label={label}>
  <span class="label">{label}</span>
  <span class="numeric-value" class:stale class:unavailable={presentation.text === '–'}>
    {#if presentation.direction === -1 || presentation.direction === 0}
      <span class="chevron chevron-left i-mdi-chevron-left" aria-hidden="true"></span>
    {/if}
    <span
      class="value"
      class:compact={presentation.text.length === 4}
      class:long={presentation.text.length >= 5}>{presentation.text}</span
    >
    {#if stackedUnit}
      <span class="unit stacked">
        <span aria-hidden="true">{stackedUnit[0]}</span>
        <span aria-hidden="true">{stackedUnit[1]}</span>
        <span class="unit-label sr-only">{presentation.unit}</span>
      </span>
    {:else if presentation.unit}
      <span class="unit" class:degree={presentation.unit === '°'}>{presentation.unit}</span>
    {/if}
    {#if presentation.direction === 1 || presentation.direction === 0}
      <span class="chevron chevron-right i-mdi-chevron-right" aria-hidden="true"></span>
    {/if}
    {#if presentation.direction === -1}
      <span class="sr-only">{m.infobox_left()}</span>
    {:else if presentation.direction === 1}
      <span class="sr-only">{m.infobox_right()}</span>
    {/if}
    {#if stale && presentation.text !== '–'}
      <span class="sr-only">{m.stale_value()}</span>
    {/if}
  </span>
</div>

<style>
  .infobox {
    display: grid;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    grid-template-rows: 1.5rem minmax(0, 1fr);
    gap: 0.125rem;
    padding: 0.375rem 0.25rem;
    background: var(--color-screen-surface);
  }

  .label {
    align-self: center;
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 0.625rem;
    font-weight: 600;
    line-height: 1.2;
    text-align: center;
    text-transform: uppercase;
  }

  .numeric-value {
    display: inline-flex;
    align-items: baseline;
    justify-content: center;
    place-self: center;
    color: var(--color-value-text);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .value {
    font-family: var(--font-numeric);
    font-size: 1.75rem;
    font-weight: 700;
    letter-spacing: -0.01em;
    line-height: 1;
  }

  .unit {
    margin-left: 0.125rem;
    color: var(--color-text-muted);
    font-size: 0.7rem;
    font-weight: 600;
    line-height: 1;
  }

  .value.compact {
    font-size: 1.375rem;
  }

  .value.long {
    font-size: 1.0625rem;
  }

  .unit.degree {
    align-self: flex-start;
    margin-top: 0.25em;
    font-size: 0.875rem;
  }

  .unit.stacked {
    display: inline-flex;
    align-self: center;
    flex-direction: column;
    align-items: center;
    gap: 0.08rem;
  }

  .unit.stacked span:first-child {
    padding-bottom: 0.08rem;
    border-bottom: 1px solid currentcolor;
  }

  .chevron {
    align-self: center;
    flex: none;
    width: 1.75rem;
    height: 1.75rem;
    margin-inline: -0.5rem;
    color: var(--color-text-muted);
  }

  .numeric-value.stale,
  .numeric-value.unavailable {
    color: var(--color-value-stale);
  }
</style>

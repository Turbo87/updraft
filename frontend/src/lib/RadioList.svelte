<script lang="ts" generics="Value extends string">
  import ResponsiveCard from './ResponsiveCard.svelte';

  type Props = {
    name: string;
    legend: string;
    options: ReadonlyArray<{ value: Value; label: string; description?: string; icon?: string }>;
    value: Value;
    hideLegend?: boolean;
    error?: string;
    onChange: (value: Value) => void;
  };

  const generatedId = $props.id();

  let { name, legend, options, value, hideLegend = false, error, onChange }: Props = $props();

  let errorId = $derived(`${generatedId}-error`);
</script>

<fieldset aria-describedby={error ? errorId : undefined}>
  <legend class:sr-only={hideLegend}>{legend}</legend>
  <ResponsiveCard error={Boolean(error)}>
    {#each options as option (option.value)}
      <label>
        <input
          type="radio"
          {name}
          value={option.value}
          checked={option.value === value}
          aria-describedby={error ? errorId : undefined}
          onchange={() => onChange(option.value)}
        />
        {#if option.icon}
          <span aria-hidden="true" class={[option.icon, 'icon']}></span>
        {/if}
        <span class="option-text">
          <span>{option.label}</span>
          {#if option.description}
            <span class="description">{option.description}</span>
          {/if}
        </span>
      </label>
    {/each}
  </ResponsiveCard>
  {#if error}
    <p id={errorId} class="error" role="alert">
      <span aria-hidden="true" class="error-icon">
        <span class="i-mdi-alert-circle-outline"></span>
      </span>
      <span>{error}</span>
    </p>
  {/if}
</fieldset>

<style>
  fieldset {
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin: 0 var(--space-1) var(--space-2);
    padding: 0;
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  label {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: var(--target-min);
    padding-block: var(--space-2);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
    color: var(--color-text);
    font: var(--text-row-label);
    font-weight: 500;
    cursor: pointer;
  }

  label + label {
    border-block-start: 1px solid var(--color-separator);
  }

  label:active {
    background: var(--color-control-surface-pressed);
  }

  input {
    flex: 0 0 auto;
    width: 1.5rem;
    height: 1.5rem;
    margin: 0;
    accent-color: var(--color-focus-ring);
  }

  .icon {
    flex: 0 0 auto;
    font-size: 1.5rem;
  }

  .option-text {
    min-width: 0;
  }

  .description {
    display: block;
    color: var(--color-text-muted);
    font: var(--text-caption);
    font-family: var(--font-numeric);
    font-weight: 500;
  }

  .error {
    display: flex;
    align-items: flex-start;
    gap: 0.375rem;
    margin: var(--space-2) var(--space-1) 0;
    color: var(--color-error-text);
    font: var(--text-caption);
    font-weight: 500;
  }

  .error-icon {
    display: inline-grid;
    flex: 0 0 auto;
    block-size: 1.5em;
    place-items: center;
  }

  .error-icon > span {
    font-size: 1.125em;
    line-height: 1;
  }
</style>

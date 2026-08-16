<script lang="ts" generics="Value extends string">
  type Props = {
    name: string;
    legend: string;
    options: ReadonlyArray<{ value: Value; label: string; description?: string; icon?: string }>;
    value: Value;
    hideLegend?: boolean;
    onChange: (value: Value) => void;
  };

  let { name, legend, options, value, hideLegend = false, onChange }: Props = $props();
</script>

<fieldset>
  <legend class:sr-only={hideLegend}>{legend}</legend>
  <div class="options">
    {#each options as option (option.value)}
      <label>
        <input
          type="radio"
          {name}
          value={option.value}
          checked={option.value === value}
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
  </div>
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

  .options {
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
  }

  label {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: var(--target-min);
    padding: var(--space-2) var(--space-5);
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
</style>

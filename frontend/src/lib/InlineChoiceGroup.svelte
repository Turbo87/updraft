<script lang="ts" generics="Value extends string">
  type Props = {
    name: string;
    legend: string;
    options: ReadonlyArray<{ value: Value; label: string }>;
    value: Value;
    onChange: (value: Value) => void;
  };

  let { name, legend, options, value, onChange }: Props = $props();
</script>

<fieldset>
  <legend>{legend}</legend>
  <div class="options" style:--choice-count={options.length}>
    {#each options as option (option.value)}
      <label class:selected={option.value === value}>
        <input
          type="radio"
          {name}
          value={option.value}
          checked={option.value === value}
          onchange={() => onChange(option.value)}
        />
        <span>{option.label}</span>
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
    display: grid;
    grid-template-columns: repeat(var(--choice-count), minmax(0, 1fr));
    gap: var(--space-2);
  }

  label {
    position: relative;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: var(--target-min);
    padding-inline: var(--space-4);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-control);
    background: var(--color-card-surface);
    box-shadow: inset 0 0 0 1px transparent;
    color: var(--color-text);
    font: var(--text-row-label);
    font-family: var(--font-numeric);
    font-weight: 500;
    cursor: pointer;
  }

  label.selected {
    border-color: var(--color-action-primary-surface);
    box-shadow: inset 0 0 0 1px var(--color-action-primary-surface);
  }

  label:active {
    background: var(--color-control-surface-pressed);
  }

  input {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    opacity: 0;
    pointer-events: none;
    clip-path: inset(50%);
  }

  input:focus-visible + span::after {
    position: absolute;
    inset: -3px;
    border: 2px solid var(--color-focus-ring);
    border-radius: calc(var(--radius-control) + 2px);
    content: '';
  }
</style>

<script lang="ts" generics="Value extends string">
  type Props = {
    name: string;
    legend: string;
    options: ReadonlyArray<{ value: Value; label: string; icon?: string }>;
    value: Value;
    hideLegend?: boolean;
    onChange: (value: Value) => void;
  };

  let { name, legend, options, value, hideLegend = false, onChange }: Props = $props();
</script>

<fieldset>
  <legend class:visually-hidden={hideLegend}>{legend}</legend>
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

  legend.visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    border: 0;
    white-space: nowrap;
    clip-path: inset(50%);
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
    padding-inline: var(--space-5);
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
</style>

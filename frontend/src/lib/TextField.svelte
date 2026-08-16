<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = Omit<HTMLInputAttributes, 'aria-describedby' | 'aria-invalid' | 'value'> & {
    label: string;
    value?: string;
    hint?: string;
    error?: string;
  };

  const generatedId = $props.id();

  let {
    label,
    value = $bindable(''),
    hint,
    error,
    id = generatedId,
    inputmode,
    disabled = false,
    class: className,
    ...attributes
  }: Props = $props();

  let descriptionId = $derived(`${id}-description`);
</script>

<label class={['text-field', { disabled }, className]}>
  <span class="label">{label}</span>
  <input
    {...attributes}
    {id}
    {inputmode}
    {disabled}
    bind:value
    aria-describedby={hint || error ? descriptionId : undefined}
    aria-invalid={error ? 'true' : undefined}
    class:numeric={inputmode === 'numeric'}
  />
  {#if error}
    <span id={descriptionId} class="error" role="alert">
      <span aria-hidden="true" class="error-icon">
        <span class="i-mdi-alert-circle-outline"></span>
      </span>
      <span>{error}</span>
    </span>
  {:else if hint}
    <span id={descriptionId} class="hint">{hint}</span>
  {/if}
</label>

<style>
  .text-field {
    display: grid;
    gap: 0.375rem;
  }

  .label {
    color: var(--color-text-muted);
    font: var(--text-row-label);
  }

  input {
    box-sizing: border-box;
    width: 100%;
    height: var(--target-min);
    padding-inline: 0.875rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-control);
    background: var(--color-screen-surface);
    color: var(--color-text);
    font: var(--text-input);
  }

  input.numeric {
    font-variant-numeric: tabular-nums;
  }

  input:focus-visible {
    border-color: var(--color-focus-ring);
    box-shadow: inset 0 0 0 1px var(--color-focus-ring);
  }

  input[aria-invalid='true'] {
    border-color: var(--color-action-destructive-surface);
    background: var(--color-danger-subtle-surface);
    box-shadow: inset 0 0 0 1px var(--color-action-destructive-surface);
  }

  .hint,
  .error {
    font: var(--text-caption);
  }

  .hint {
    color: var(--color-text-muted);
  }

  .error {
    display: flex;
    align-items: flex-start;
    gap: 0.375rem;
    color: var(--color-danger-subtle-text);
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

  .disabled .label {
    color: var(--color-disabled-text);
  }

  input:disabled {
    border-color: var(--color-border);
    background: var(--color-disabled-surface);
    box-shadow: none;
    color: var(--color-disabled-text);
    cursor: not-allowed;
  }
</style>

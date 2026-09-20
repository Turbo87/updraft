<script lang="ts" generics="Value extends string">
  import type { ComponentProps } from 'svelte';

  import RadioList from './RadioList.svelte';

  type Props = Pick<
    ComponentProps<typeof RadioList<Value>>,
    'name' | 'legend' | 'options' | 'value'
  > & {
    onChange: (value: Value) => Promise<void>;
    errorMessage: () => string;
  };

  let { value, onChange, errorMessage, ...props }: Props = $props();
  let pending = $state<Value | null>(null);
  let error = $state<string>();

  async function select(value: Value) {
    pending = value;
    error = undefined;
    try {
      await onChange(value);
    } catch {
      error = errorMessage();
    } finally {
      pending = null;
    }
  }
</script>

<RadioList
  {...props}
  value={pending ?? value}
  disabled={pending !== null}
  {error}
  onChange={select}
/>

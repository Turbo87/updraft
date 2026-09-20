<script lang="ts">
  import type { Pathname } from '$app/types';
  import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';

  import { resolve } from '$app/paths';

  import NavigateButton from '$lib/NavigateButton.svelte';
  import PinTargetButton from '$lib/PinTargetButton.svelte';

  type Props = {
    target: NavigationTarget;
    label: string;
    href: Pathname;
    onPinFailure?: (retry: () => Promise<boolean>) => void;
  };
  let { target, label, href, onPinFailure }: Props = $props();
</script>

<div class="row">
  <a href={resolve(href)}>{label}</a>
  <NavigateButton {target} iconOnly />
  <PinTargetButton {target} iconOnly onFailure={onPinFailure} />
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding-block: var(--space-2);
  }
  a {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>

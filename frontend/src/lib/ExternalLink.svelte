<script module lang="ts">
  import { isTauri } from '@tauri-apps/api/core';

  const inTauri = isTauri();
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes } from 'svelte/elements';

  import { openUrl } from '@tauri-apps/plugin-opener';

  type Props = Omit<HTMLAnchorAttributes, 'children' | 'href'> & {
    children: Snippet;
    href: string;
  };
  type AnchorClickEvent = Parameters<NonNullable<HTMLAnchorAttributes['onclick']>>[0];

  let {
    children,
    href,
    onclick,
    rel = 'noopener noreferrer',
    target = '_blank',
    ...attributes
  }: Props = $props();

  function open(event: AnchorClickEvent) {
    onclick?.(event);
    if (!inTauri || event.defaultPrevented) return;

    event.preventDefault();
    void openUrl(href);
  }
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve -- External URLs open in a browser. -->
<a {...attributes} {href} {rel} {target} onclick={inTauri || onclick ? open : undefined}>
  {@render children()}
</a>
<!-- eslint-enable svelte/no-navigation-without-resolve -->

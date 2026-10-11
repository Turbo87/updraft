<script module lang="ts">
  // Detail pages below Navigation link back here, so the origin outlives this component.
  let openedFromMainMenu = $state(false);
</script>

<script lang="ts">
  import { afterNavigate } from '$app/navigation';
  import { resolve } from '$app/paths';

  import { getAppContext } from '#lib/app-context.js';
  import Button from '#lib/Button.svelte';
  import { targetsMatch } from '#lib/navigation-target.js';
  import { navigationLabel } from '#lib/navigation.js';
  import NavigationSelectionRow from '#lib/NavigationSelectionRow.svelte';
  import { m } from '#lib/paraglide/messages.js';
  import ScreenScaffold from '#lib/ScreenScaffold.svelte';
  import StopNavigationButton from '#lib/StopNavigationButton.svelte';

  const { navigation } = getAppContext();

  afterNavigate(({ from, type }) => {
    // SvelteKit passes a `from` target with a null URL on the first page load.
    if (type === 'enter') openedFromMainMenu = false;
    else if (from?.url.pathname === resolve('/menu')) openedFromMainMenu = true;
    else if (from?.url.pathname === resolve('/')) openedFromMainMenu = false;
  });

  let retryPin = $state.raw<(() => Promise<boolean>) | null>(null);
  let busy = $state(false);
  function pinFailed(retry: () => Promise<boolean>) {
    retryPin = retry;
  }
  async function retry() {
    busy = true;
    try {
      if (await retryPin?.()) retryPin = null;
    } catch {
      /* Keep the retry action available. */
    } finally {
      busy = false;
    }
  }
</script>

<ScreenScaffold
  title={m.navigation_heading()}
  backHref={openedFromMainMenu ? resolve('/menu') : resolve('/')}
  backLabel={openedFromMainMenu ? m.back_to_main_menu() : m.flight_view()}
>
  {#if navigation.current}
    <h2>{m.navigation_current()}</h2>
    <NavigationSelectionRow
      onPinFailure={pinFailed}
      target={navigation.current.target}
      label={navigationLabel(navigation.current)}
      href={navigation.current.target.type === 'task'
        ? resolve('/task')
        : resolve('/navigation/current')}
    />
  {/if}
  {#if navigation.current?.target.type !== 'task' && !navigation.pins.some((pin) => pin.navigation.target.type === 'task')}
    {#if navigation.task.points.length > 0}<NavigationSelectionRow
        target={{ type: 'task' }}
        label={m.task_heading()}
        href={resolve('/task')}
        onPinFailure={pinFailed}
      />
    {:else}<a href={resolve('/task')}>{m.task_heading()}</a>{/if}
  {/if}
  <h2>{m.pins_details()}</h2>
  {#each navigation.pins.filter((pin) => !pin.primary) as pin (pin.id)}
    <NavigationSelectionRow
      onPinFailure={pinFailed}
      target={pin.navigation.target}
      label={navigationLabel(pin.navigation)}
      href={pin.navigation.target.type === 'task'
        ? resolve('/task')
        : resolve('/pinned-targets/[id]', { id: String(pin.id) })}
    />
  {/each}
  <h2>{m.navigation_recent()}</h2>
  {#each navigation.recents as target, index (JSON.stringify(target))}
    {#if !navigation.pins.some( (pin) => targetsMatch(pin.navigation.target, target) ) && !(navigation.current && targetsMatch(navigation.current.target, target))}
      <NavigationSelectionRow
        onPinFailure={pinFailed}
        {target}
        label={navigationLabel({ target, traffic: null })}
        href={resolve('/navigation/recent/[index]', { index: String(index) })}
      />
    {/if}
  {/each}
  {#if retryPin}<p role="alert">{m.pins_save_failed()}</p>
    <Button onclick={retry} loading={busy}>{m.retry()}</Button>{/if}
  {#snippet actions()}<StopNavigationButton />{/snippet}
</ScreenScaffold>

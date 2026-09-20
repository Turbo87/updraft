<script lang="ts">
  import type { FeatureCollection, Point } from 'geojson';
  import type { GeoJSONSource } from 'maplibre-gl';
  import type { TaskCommand } from '$lib/protocol/generated/TaskCommand';
  import type { WaypointFeature, WaypointProperties } from '$lib/waypoints';

  import { getAppContext } from '$lib/app-context';
  import Button from '$lib/Button.svelte';
  import NavigateButton from '$lib/NavigateButton.svelte';
  import { navigationLabel } from '$lib/navigation';
  import { waypointTarget } from '$lib/navigation-target';
  import { m } from '$lib/paraglide/messages';
  import PinTargetButton from '$lib/PinTargetButton.svelte';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { client, navigation, mapState, waypoints } = getAppContext();
  const task = $derived(navigation.task);
  let available = $state.raw<WaypointFeature[]>([]);
  let query = $state('');
  let busy = $state(false);
  let error = $state(false);
  let notSaved = $state(false);
  let loadError = $state(false);
  let retryCount = $state(0);
  $effect(() => {
    let map = mapState.map;
    void waypoints.current.generation;
    void retryCount;
    let active = true;
    available = [];
    loadError = false;
    async function load() {
      let source = map?.getSource<GeoJSONSource>('waypoints');
      if (!source || !map?.isSourceLoaded('waypoints')) return;
      try {
        let data = (await source.getData()) as FeatureCollection<Point, WaypointProperties>;
        if (active) available = data.features;
      } catch {
        if (active) loadError = true;
      }
    }
    map?.on('sourcedata', load);
    void load();
    return () => {
      active = false;
      map?.off('sourcedata', load);
    };
  });
  const matches = $derived(
    available
      .filter((point) =>
        point.properties.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
      )
      .slice(0, 30),
  );
  async function change(command: TaskCommand) {
    busy = true;
    error = false;
    try {
      notSaved = !(await client.changeTask(command));
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
  async function save() {
    busy = true;
    try {
      notSaved = !(await client.saveTask());
    } catch {
      notSaved = true;
    } finally {
      busy = false;
    }
  }
</script>

<ScreenScaffold title={m.task_heading()} backHref="/navigation" backLabel={m.navigation_heading()}>
  <p>
    {task.status === 'running'
      ? m.task_running()
      : task.status === 'completed'
        ? m.task_completed()
        : m.task_stopped()}
  </p>
  {#if task.start}<p>
      {m.task_start()}: {task.start.unixMilliseconds === null
        ? '—'
        : new Date(task.start.unixMilliseconds).toLocaleTimeString(undefined, { timeZone: 'UTC' })} UTC
    </p>{/if}
  {#if task.finish}<p>
      {m.task_finish()}: {task.finish.unixMilliseconds === null
        ? '—'
        : new Date(task.finish.unixMilliseconds).toLocaleTimeString(undefined, { timeZone: 'UTC' })} UTC
    </p>{/if}
  <ol>
    {#each task.points as point, index (point.id)}
      <li aria-current={point.id === task.current ? 'step' : undefined}>
        <span
          >{index === 0
            ? m.task_start()
            : index === task.points.length - 1
              ? m.task_finish()
              : m.task_turnpoint()} · 500 m</span
        >
        <div class="point">
          <Button
            variant="secondary"
            onclick={() => change({ type: 'select', id: point.id })}
            disabled={busy || task.points.length < 2}
            >{navigationLabel({ target: point.target, traffic: null })}</Button
          >
          <Button
            aria-label={m.task_up()}
            disabled={busy || index === 0}
            onclick={() => change({ type: 'move', id: point.id, index: index - 1 })}>↑</Button
          >
          <Button
            aria-label={m.task_down()}
            disabled={busy || index === task.points.length - 1}
            onclick={() => change({ type: 'move', id: point.id, index: index + 1 })}>↓</Button
          >
          <Button
            aria-label={m.task_remove()}
            disabled={busy || (task.status === 'running' && task.points.length <= 2)}
            onclick={() => change({ type: 'remove', id: point.id })}>×</Button
          >
        </div>
      </li>
    {/each}
  </ol>
  <h2>{m.task_add()}</h2>
  <label>{m.task_search()}<input type="search" bind:value={query} /></label>
  {#if loadError}<p role="alert">{m.waypoint_load_failed()}</p>
    <Button onclick={() => retryCount++}>{m.retry()}</Button>{/if}
  {#each matches as waypoint (waypoint.properties.id)}
    <Button
      variant="secondary"
      disabled={busy}
      onclick={() => change({ type: 'add', target: waypointTarget(waypoint) })}
      >{waypoint.properties.name}</Button
    >
  {/each}
  {#if error || notSaved}<p role="alert">{m.task_failed()}</p>{/if}
  {#if notSaved}<Button onclick={save} loading={busy}>{m.retry()}</Button>{/if}
  {#snippet actions()}
    {#if task.points.length >= 2}<NavigateButton target={{ type: 'task' }} />{/if}
    {#if task.points.length}<PinTargetButton target={{ type: 'task' }} />{/if}
    {#if task.status === 'running'}<Button onclick={() => change({ type: 'stop' })} loading={busy}
        >{m.task_stop()}</Button
      >{/if}
  {/snippet}
</ScreenScaffold>

<style>
  li {
    margin-block: var(--space-3);
  }
  li[aria-current] {
    border-left: 3px solid currentColor;
    padding-left: var(--space-2);
  }
  .point {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  input {
    display: block;
    width: 100%;
  }
</style>

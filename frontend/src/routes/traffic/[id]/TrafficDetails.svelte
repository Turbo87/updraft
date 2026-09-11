<script lang="ts">
  import type { Locale } from '$lib/protocol/generated/Locale';
  import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
  import type { TrafficAlarmLevel } from '$lib/protocol/generated/TrafficAlarmLevel';
  import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { InstrumentsStore } from '$lib/stores/instruments.svelte';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { onMount } from 'svelte';

  import Card from '$lib/Card.svelte';
  import { calculateDistanceAndBearing } from '$lib/geographic-position';
  import { m } from '$lib/paraglide/messages.js';
  import ResponsiveCard from '$lib/ResponsiveCard.svelte';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';
  import StatusPill from '$lib/StatusPill.svelte';
  import { convertAltitude, convertDistance, convertVerticalSpeed } from '$lib/units';
  import ValueTile from '$lib/ValueTile.svelte';
  import {
    formatTrafficAlarmLevel,
    formatTrafficId,
    formatTrafficType,
  } from '../../nearby/[latitude]/[longitude]/nearby-traffic';

  type RetainedTarget = { target: PublishedTrafficTarget; available: boolean } | null;
  type StatusTone = 'normal' | 'caution' | 'success' | 'warning' | 'danger';

  type Props = {
    backLabel: string;
    id: string;
    instruments: InstrumentsStore;
    locale: Locale;
    onBack: (event: MouseEvent) => void;
    traffic: TrafficStore;
    units: UnitSettings;
  };

  let { backLabel, id, instruments, locale, onBack, traffic, units }: Props = $props();

  let retainedTarget = $state.raw<RetainedTarget>();
  let ownshipRelation = $derived(
    retainedTarget && instruments.current.gps?.position
      ? calculateDistanceAndBearing(
          instruments.current.gps.position,
          retainedTarget.target.position,
        )
      : null,
  );
  let valueStale = $derived(
    retainedTarget ? retainedTarget.target.stale || !retainedTarget.available : true,
  );

  let flarmnetFields = $derived.by(() => {
    let record = retainedTarget?.target.flarmnet;
    if (!record) return [];
    return [
      [m.callsign_label(), record.callSign],
      [m.registration_label(), record.registration],
      [m.aircraft_model_label(), record.planeType],
      [m.pilot_label(), record.pilotName],
      [m.airfield_label(), record.airfield],
      [m.frequency_label(), record.frequency],
      [m.flarm_id_label(), record.flarmId],
    ];
  });

  function createRetainedTarget(): RetainedTarget {
    let target = traffic.current.get(id);
    return target ? { target, available: true } : null;
  }

  function handleTrafficUpdate(
    _update: TrafficUpdate,
    currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
  ) {
    let updatedTarget = currentTargets.get(id);
    if (updatedTarget) {
      retainedTarget = { target: updatedTarget, available: true };
    } else if (retainedTarget) {
      retainedTarget = { ...retainedTarget, available: false };
    }
  }

  function alarmTone(alarmLevel: TrafficAlarmLevel): StatusTone {
    switch (alarmLevel) {
      case 'unknown':
      case 'none':
        return 'normal';
      case 'low':
        return 'caution';
      case 'important':
        return 'warning';
      case 'urgent':
        return 'danger';
    }
  }

  function stateLabel(target: RetainedTarget): string {
    if (!target?.available) return m.unavailable_value();
    return target.target.stale ? m.stale_value() : m.fresh_value();
  }

  function stateTone(target: RetainedTarget): StatusTone {
    if (!target?.available) return 'normal';
    return target.target.stale ? 'caution' : 'success';
  }

  function formatPosition(target: PublishedTrafficTarget): string {
    let { latitudeDegrees, longitudeDegrees } = target.position;
    let latitudeHemisphere = latitudeDegrees >= 0 ? 'N' : 'S';
    let longitudeHemisphere = longitudeDegrees >= 0 ? 'E' : 'W';
    return `${Math.abs(latitudeDegrees).toFixed(5)}° ${latitudeHemisphere}, ${Math.abs(longitudeDegrees).toFixed(5)}° ${longitudeHemisphere}`;
  }

  function formatAltitude(meters: number): string {
    return `${convertAltitude(meters, units.altitude).toFixed(0)} ${units.altitude}`;
  }

  function formatClimb(metersPerSecond: number | undefined): string {
    if (metersPerSecond === undefined) return '—';
    let digits = units.verticalSpeed === 'ft/min' ? 0 : 1;
    let value = new Intl.NumberFormat(locale, {
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
      signDisplay: 'exceptZero',
      useGrouping: false,
    }).format(convertVerticalSpeed(metersPerSecond, units.verticalSpeed));
    return `${value} ${units.verticalSpeed}`;
  }

  function formatRelativeAltitude(target: PublishedTrafficTarget): string {
    let ownshipAltitude = instruments.current.gps?.altitudeMeters;
    if (
      target.altitudeMslMeters === null ||
      ownshipAltitude === null ||
      ownshipAltitude === undefined
    ) {
      return '—';
    }

    let altitude = Math.round(
      convertAltitude(target.altitudeMslMeters - ownshipAltitude, units.altitude),
    );
    let value = altitude > 0 ? `+${altitude}` : altitude < 0 ? `−${Math.abs(altitude)}` : '0';
    return `${value} ${units.altitude}`;
  }

  onMount(() => {
    retainedTarget = createRetainedTarget();
    return traffic.subscribe(handleTrafficUpdate);
  });
</script>

<ScreenScaffold {backLabel} {onBack} title={formatTrafficId(id)}>
  {#if !traffic.initialized || retainedTarget === undefined}
    <Card>
      <p class="empty-state">{m.traffic_details_loading()}</p>
    </Card>
  {:else if retainedTarget === null}
    <Card>
      <p class="empty-state">{m.traffic_not_found()}</p>
    </Card>
  {:else}
    {@const target = retainedTarget.target}
    {@const relativeAltitude = formatRelativeAltitude(target)}
    <Card>
      <div class="summary">
        <ValueTile
          label={m.distance_label()}
          stale={valueStale || !ownshipRelation}
          unit={ownshipRelation ? units.distance : undefined}
          value={ownshipRelation
            ? convertDistance(ownshipRelation.distanceMeters, units.distance).toFixed(1)
            : '—'}
        />
        <ValueTile
          label={m.bearing_label()}
          stale={valueStale || !ownshipRelation}
          unit={ownshipRelation ? '°' : undefined}
          value={ownshipRelation ? ownshipRelation.bearingDegrees.toFixed(0).padStart(3, '0') : '—'}
        />
        <div class="alarm-summary">
          <span>{m.alarm_level_label()}</span>
          <StatusPill
            icon={target.alarmLevel === 'none' ? undefined : 'i-mdi-alert'}
            label={formatTrafficAlarmLevel(target.alarmLevel, locale)}
            tone={alarmTone(target.alarmLevel)}
          />
        </div>
      </div>
    </Card>

    <section>
      <h2>{m.traffic_target_heading()}</h2>
      <ResponsiveCard>
        <dl>
          <div>
            <dt>{m.traffic_id_label()}</dt>
            <dd>{target.id}</dd>
          </div>
          <div>
            <dt>{m.traffic_type_label()}</dt>
            <dd>{formatTrafficType(target.trafficType, locale)}</dd>
          </div>
          <div>
            <dt>{m.state_label()}</dt>
            <dd>
              <StatusPill label={stateLabel(retainedTarget)} tone={stateTone(retainedTarget)} />
            </dd>
          </div>
        </dl>
      </ResponsiveCard>
    </section>

    {#if target.flarmnet}
      <section aria-labelledby="flarmnet-heading">
        <h2 id="flarmnet-heading">FlarmNet</h2>
        <ResponsiveCard>
          <dl>
            {#each flarmnetFields as [label, value] (label)}
              {#if value}
                <div>
                  <dt>{label}</dt>
                  <dd>{value}</dd>
                </div>
              {/if}
            {/each}
          </dl>
        </ResponsiveCard>
      </section>
    {/if}

    <section>
      <h2>{m.position_label()}</h2>
      <ResponsiveCard>
        <dl>
          <div>
            <dt>{m.position_label()}</dt>
            <dd class="numeric">{formatPosition(target)}</dd>
          </div>
          <div>
            <dt>{m.altitude_label()}</dt>
            <dd class="altitude numeric">
              <span>
                {target.altitudeMslMeters === null
                  ? '—'
                  : `${formatAltitude(target.altitudeMslMeters)} MSL`}
              </span>
              <span class:stale={valueStale || relativeAltitude === '—'} class="relative-altitude">
                {relativeAltitude}
              </span>
            </dd>
          </div>
          {#each [{ label: m.climb_20s_label(), value: target.climb?.average20s }, { label: m.climb_30s_label(), value: target.climb?.average30s }, { label: m.climb_ema_label(), value: target.climb?.normalizedEma }, { label: m.climb_smoothed_20s_label(), value: target.climb?.smoothed20s }] as estimate (estimate.label)}
            <div>
              <dt>{estimate.label}</dt>
              <dd class="climb numeric" class:stale={valueStale || estimate.value === undefined}>
                {formatClimb(estimate.value)}
              </dd>
            </div>
          {/each}
          <div>
            <dt>{m.track_label()}</dt>
            <dd class="numeric">
              {target.trackDegrees === null ? '—' : `${target.trackDegrees.toFixed(0)}° true`}
            </dd>
          </div>
        </dl>
      </ResponsiveCard>
    </section>
  {/if}
</ScreenScaffold>

<style>
  .empty-state {
    margin: 0;
    padding: var(--space-5);
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  .summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1px;
    background: var(--color-separator);
  }

  .alarm-summary {
    display: flex;
    grid-column: 1 / -1;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--color-card-surface);
  }

  .alarm-summary > span {
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  section {
    margin-block-start: var(--space-6);
  }

  h2 {
    margin: 0 var(--space-1) var(--space-2);
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  dl {
    margin: 0;
  }

  dl > div {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    min-height: var(--target-min);
    padding-block: var(--space-2);
    padding-inline: calc(var(--space-5) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
  }

  dl > div + div {
    border-block-start: 1px solid var(--color-separator);
  }

  dt {
    flex: 0 0 auto;
    font: var(--text-row-label);
  }

  dd {
    min-width: 0;
    margin: 0;
    color: var(--color-text-muted);
    font: var(--text-row-detail);
    text-align: end;
  }

  dd.numeric {
    color: var(--color-value-text);
    font-family: var(--font-numeric);
    font-variant-numeric: tabular-nums;
  }

  .altitude {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .relative-altitude {
    color: var(--color-success-text);
  }

  .relative-altitude.stale,
  .climb.stale {
    color: var(--color-value-stale);
  }
</style>

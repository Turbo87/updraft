<script lang="ts">
  import type * as GeoJSON from 'geojson';
  import type { GeoJSONSource, Map, MapEventType } from 'maplibre-gl';
  import type { AirspaceProperties } from '$lib/airspace';
  import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
  import type { Locale } from '$lib/protocol/generated/Locale';

  import { onMount } from 'svelte';

  import { m } from '$lib/paraglide/messages.js';
  import {
    formatAirspaceActivity,
    formatAirspaceClass,
    formatAirspaceDateTime,
    formatAirspaceDay,
    formatAirspaceLimit,
    formatAirspaceTime,
    formatAirspaceType,
  } from './airspace-format';

  type QueryState =
    | { type: 'loading' }
    | { type: 'failed' }
    | { type: 'notFound' }
    | { type: 'ready'; properties: AirspaceProperties };

  let {
    altitudeUnit,
    id,
    locale,
    map,
  }: { altitudeUnit: AltitudeUnit; id: number; locale: Locale; map: Map } = $props();
  let queryState = $state.raw<QueryState>({ type: 'loading' });
  let activeRequest = 0;
  let querying = false;

  async function queryAirspace() {
    if (queryState.type !== 'loading' || querying) return;

    let source = map.getSource<GeoJSONSource>('airspace');
    if (!source || !map.isSourceLoaded('airspace')) return;

    let request = ++activeRequest;
    querying = true;
    try {
      let data = (await source.getData()) as GeoJSON.FeatureCollection;
      if (request !== activeRequest) return;

      let feature = data.features.find((candidate) => candidate.id === id);
      queryState = feature
        ? { type: 'ready', properties: feature.properties as AirspaceProperties }
        : { type: 'notFound' };
    } catch {
      if (request === activeRequest) queryState = { type: 'failed' };
    } finally {
      if (request === activeRequest) querying = false;
    }
  }

  function handleMapError(event: MapEventType['error']) {
    if ('sourceId' in event && event.sourceId === 'airspace') {
      activeRequest += 1;
      querying = false;
      queryState = { type: 'failed' };
    }
  }

  function retry() {
    queryState = { type: 'loading' };
    queryAirspace();
  }

  function booleanValue(value: boolean): string {
    return value ? m.yes_value() : m.no_value();
  }

  onMount(() => {
    map.on('styledata', queryAirspace);
    map.on('sourcedata', queryAirspace);
    map.on('error', handleMapError);
    queryAirspace();

    return () => {
      activeRequest += 1;
      map.off('styledata', queryAirspace);
      map.off('sourcedata', queryAirspace);
      map.off('error', handleMapError);
    };
  });
</script>

{#if queryState.type === 'loading'}
  <p>{m.airspace_details_loading()}</p>
{:else if queryState.type === 'failed'}
  <p>{m.airspace_details_failed()}</p>
  <button type="button" onclick={retry}>{m.retry()}</button>
{:else if queryState.type === 'notFound'}
  <p>{m.airspace_not_found()}</p>
{:else}
  {@const properties = queryState.properties}
  {@const countries = Array.isArray(properties.country)
    ? properties.country
    : properties.country
      ? [properties.country]
      : []}
  <h1>{properties.name ?? m.unnamed_airspace()}</h1>

  <section>
    <h2>{m.classification_heading()}</h2>
    <dl>
      <dt>{m.airspace_type_label()}</dt>
      <dd>{formatAirspaceType(properties.type, locale)}</dd>
      {#if properties.icaoClass !== 8}
        <dt>{m.icao_class_label()}</dt>
        <dd>{formatAirspaceClass(properties.icaoClass, locale)}</dd>
      {/if}
      {#if properties.activity !== undefined}
        <dt>{m.activity_label()}</dt>
        <dd>{formatAirspaceActivity(properties.activity, locale)}</dd>
      {/if}
    </dl>
  </section>

  <section>
    <h2>{m.vertical_limits_heading()}</h2>
    <dl>
      <dt>{m.upper_limit_label()}</dt>
      <dd>
        <!-- eslint-disable-next-line prefer-let/prefer-let -- Svelte element declarations require `const`. -->
        {const upperLimit = formatAirspaceLimit(properties.upperLimit, altitudeUnit)}
        {properties.upperLimitMax
          ? m.airspace_limit_maximum({
              limit: upperLimit,
              maximum: formatAirspaceLimit(properties.upperLimitMax, altitudeUnit),
            })
          : upperLimit}
      </dd>
      <dt>{m.lower_limit_label()}</dt>
      <dd>
        <!-- eslint-disable-next-line prefer-let/prefer-let -- Svelte element declarations require `const`. -->
        {const lowerLimit = formatAirspaceLimit(properties.lowerLimit, altitudeUnit)}
        {properties.lowerLimitMin
          ? m.airspace_limit_minimum({
              limit: lowerLimit,
              minimum: formatAirspaceLimit(properties.lowerLimitMin, altitudeUnit),
            })
          : lowerLimit}
      </dd>
    </dl>
  </section>

  {#if countries.length > 0}
    <section>
      <h2>{m.countries_heading()}</h2>
      <ul>
        {#each countries as country (country)}
          <li>{country}</li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if properties.frequencies?.length || properties.transponderSettings?.length}
    <section>
      <h2>{m.communications_heading()}</h2>
      {#if properties.frequencies?.length}
        <dl>
          {#each properties.frequencies as frequency (frequency)}
            <dt>{frequency.name ?? m.frequency_label()}</dt>
            <dd>{frequency.value} MHz</dd>
            {#if frequency.primary !== undefined}
              <dt>{m.primary_label()}</dt>
              <dd>{booleanValue(frequency.primary)}</dd>
            {/if}
            {#if frequency.remarks}
              <dt>{m.remarks_label()}</dt>
              <dd>{frequency.remarks}</dd>
            {/if}
          {/each}
        </dl>
      {/if}
      {#if properties.transponderSettings?.length}
        <dl>
          {#each properties.transponderSettings as setting (setting)}
            <dt>{m.transponder_code_label()}</dt>
            <dd>{setting.code}</dd>
            <dt>{m.primary_label()}</dt>
            <dd>{booleanValue(setting.primary)}</dd>
            {#if setting.remarks}
              <dt>{m.remarks_label()}</dt>
              <dd>{setting.remarks}</dd>
            {/if}
          {/each}
        </dl>
      {/if}
    </section>
  {/if}

  {#if properties.onDemand !== undefined || properties.onRequest !== undefined || properties.byNotam !== undefined || properties.specialAgreement !== undefined || properties.requestCompliance !== undefined || properties.activeFrom || properties.activeUntil || properties.hoursOfOperation}
    <section>
      <h2>{m.activation_heading()}</h2>
      <dl>
        {#if properties.onDemand !== undefined}
          <dt>{m.on_demand_label()}</dt>
          <dd>{booleanValue(properties.onDemand)}</dd>
        {/if}
        {#if properties.onRequest !== undefined}
          <dt>{m.on_request_label()}</dt>
          <dd>{booleanValue(properties.onRequest)}</dd>
        {/if}
        {#if properties.byNotam !== undefined}
          <dt>{m.by_notam_label()}</dt>
          <dd>{booleanValue(properties.byNotam)}</dd>
        {/if}
        {#if properties.specialAgreement !== undefined}
          <dt>{m.special_agreement_label()}</dt>
          <dd>{booleanValue(properties.specialAgreement)}</dd>
        {/if}
        {#if properties.requestCompliance !== undefined}
          <dt>{m.request_compliance_label()}</dt>
          <dd>{booleanValue(properties.requestCompliance)}</dd>
        {/if}
        {#if properties.activeFrom}
          <dt>{m.active_from_label()}</dt>
          <dd>{formatAirspaceDateTime(properties.activeFrom, locale)}</dd>
        {/if}
        {#if properties.activeUntil}
          <dt>{m.active_until_label()}</dt>
          <dd>{formatAirspaceDateTime(properties.activeUntil, locale)}</dd>
        {/if}
      </dl>

      {#if properties.hoursOfOperation}
        <h3>{m.operating_hours_heading()}</h3>
        {#each properties.hoursOfOperation.operatingHours as period (period)}
          <dl>
            <dt>{formatAirspaceDay(period.dayOfWeek, locale)}</dt>
            <dd></dd>
            <dt>{m.start_time_label()}</dt>
            <dd>
              {#if period.sunrise}
                {m.sunrise_value()}
              {:else if period.startTime}
                {formatAirspaceTime(period.startTime)}
              {:else}
                {m.unspecified_time_value()}
              {/if}
            </dd>
            <dt>{m.end_time_label()}</dt>
            <dd>
              {#if period.sunset}
                {m.sunset_value()}
              {:else if period.endTime}
                {formatAirspaceTime(period.endTime)}
              {:else}
                {m.unspecified_time_value()}
              {/if}
            </dd>
            <dt>{m.by_notam_label()}</dt>
            <dd>{booleanValue(period.byNotam)}</dd>
            <dt>{m.public_holidays_excluded_label()}</dt>
            <dd>{booleanValue(period.publicHolidaysExcluded)}</dd>
            {#if period.remarks}
              <dt>{m.remarks_label()}</dt>
              <dd>{period.remarks}</dd>
            {/if}
          </dl>
        {/each}
        {#if properties.hoursOfOperation.remarks}
          <dl>
            <dt>{m.remarks_label()}</dt>
            <dd>{properties.hoursOfOperation.remarks}</dd>
          </dl>
        {/if}
      {/if}
    </section>
  {/if}

  {#if properties.remarks}
    <section>
      <h2>{m.remarks_heading()}</h2>
      <p>{properties.remarks}</p>
    </section>
  {/if}
{/if}

<style>
  section {
    margin-block-start: 1.5rem;
  }

  h1,
  h2,
  h3 {
    margin-block-end: 0.5rem;
  }

  dl {
    display: grid;
    grid-template-columns: max-content auto;
    gap: 0.5rem 1rem;
  }

  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
</style>

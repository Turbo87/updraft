<script lang="ts">
  import type { UpdraftClient } from '#lib/client/index.js';
  import type { FlightBoundaryQuestion } from '#lib/protocol/generated/FlightBoundaryQuestion.js';
  import type { DistanceUnit } from '#lib/units.js';

  import ConfirmDialog from '#lib/ConfirmDialog.svelte';
  import { m } from '#lib/paraglide/messages.js';
  import { getLocale } from '#lib/paraglide/runtime.js';
  import { convertDistance } from '#lib/units.js';

  type Props = {
    question: FlightBoundaryQuestion | null;
    distanceUnit: DistanceUnit;
    client: Pick<UpdraftClient, 'startNewFlight' | 'continueFlight'>;
  };

  let { question, distanceUnit, client }: Props = $props();

  let pending = $state(false);
  let failed = $state(false);

  const durationUnits = [
    ['day', 24 * 60 * 60 * 1000],
    ['hour', 60 * 60 * 1000],
    ['minute', 60 * 1000],
    ['second', 1000],
  ] as const;

  function formatDuration(milliseconds: number): string {
    let [unit, length] =
      durationUnits.find(([, length]) => milliseconds >= length) ?? durationUnits[3];
    let format = new Intl.NumberFormat(getLocale(), {
      style: 'unit',
      unit,
      unitDisplay: 'long',
      maximumFractionDigits: 0,
    });
    return format.format(Math.floor(milliseconds / length));
  }

  function describe(question: FlightBoundaryQuestion): string {
    switch (question.cause) {
      case 'positionJump': {
        let number = new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0 });
        let distance = `${number.format(convertDistance(question.meters, distanceUnit))} ${distanceUnit}`;
        return m.flight_boundary_position_jump({ distance });
      }
      case 'fixUtcJump': {
        let duration = formatDuration(Math.abs(question.milliseconds));
        return question.milliseconds > 0
          ? m.flight_boundary_time_jump_forward({ duration })
          : m.flight_boundary_time_jump_back({ duration });
      }
    }
  }

  async function answer(command: () => Promise<void>) {
    pending = true;
    failed = false;
    try {
      await command();
    } catch {
      failed = true;
    } finally {
      pending = false;
    }
  }
</script>

{#if question}
  <!-- The core closes the question, so a local close has no effect. -->
  <ConfirmDialog
    bind:open={() => true, () => {}}
    title={m.flight_boundary_title()}
    description={`${describe(question)} ${m.flight_boundary_hint()}`}
    cancelLabel={m.continue_flight()}
    confirmLabel={m.new_flight()}
    {pending}
    error={failed ? m.flight_boundary_answer_failed() : undefined}
    onCancel={() => void answer(() => client.continueFlight())}
    onConfirm={() => void answer(() => client.startNewFlight())}
  />
{/if}

import type { DerivedInstruments } from '$lib/protocol/generated/DerivedInstruments';
import type { Instruments } from '$lib/protocol/generated/Instruments';
import type { Topic } from '$lib/protocol/generated/Topic';

/** Every derived value absent, for a test that names only a few. */
export const EMPTY_DERIVED_INSTRUMENTS: DerivedInstruments = {
  rawVerticalSpeed: null,
  verticalSpeed: null,
  vario: null,
  wind: null,
  airspeed: null,
  heading: null,
  altitude: null,
  bank: null,
  netto: null,
  relativeVario: null,
};

/**
 * Every instrument absent, which is what a client sees before the first
 * fix. Exported so that a test can name the few values it cares about
 * and leave the rest empty.
 */
export const EMPTY_INSTRUMENTS: Instruments = {
  gps: null,
  pressureAltitude: null,
  trueAirspeed: null,
  derived: null,
};

/**
 * Holds the latest instruments topic.
 *
 * Topics arrive whole, so the store replaces rather than merges and the
 * view is a pure function of the last message received.
 */
export class InstrumentsStore {
  current = $state.raw<Instruments>(EMPTY_INSTRUMENTS);

  apply(topic: Topic): void {
    if (topic.topic !== 'instruments') return;

    this.current = topic.value;
  }
}

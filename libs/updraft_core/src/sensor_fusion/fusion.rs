use super::estimator::Estimator;
use super::vario::SampleAcceptance;
use crate::ownship::{DomainState, Selected};
use crate::signal_state::SignalState;
use crate::topic::{DerivedInstruments, SpeedInstrument};
use updraft_units::{PressureAltitude, Speed};

/// Selected sensor states for one core time advancement.
///
/// The complete input keeps estimator updates independent of source-selection
/// call order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FusionInputs {
    pub true_airspeed: DomainState<Speed>,
    pub pressure_altitude: DomainState<PressureAltitude>,
}

/// Connects selected sensor domains to the flight-data estimator.
///
/// This layer owns source identity, input continuity, freshness, and protocol
/// projection. It resets dependent estimator state when input continuity breaks.
#[derive(Clone, Debug, Default)]
pub struct SensorFusion {
    estimator: Estimator,
    air_speed: Option<Selected<Speed>>,
    pressure_altitude: Option<Selected<PressureAltitude>>,
    raw_vertical_speed: SignalState<Speed>,
    vertical_speed: SignalState<Speed>,
}

impl SensorFusion {
    /// Applies one coherent set of selected sensor states.
    pub fn update(&mut self, inputs: FusionInputs) {
        self.update_true_airspeed(inputs.true_airspeed);
        self.update_pressure_altitude(inputs.pressure_altitude);
    }

    fn update_true_airspeed(&mut self, state: DomainState<Speed>) {
        let DomainState::Current(selected) = state else {
            self.air_speed = None;
            return;
        };
        if self.air_speed == Some(selected) {
            return;
        }
        self.air_speed = Some(selected);
    }

    fn update_pressure_altitude(&mut self, state: DomainState<PressureAltitude>) {
        let DomainState::Current(selected) = state else {
            self.raw_vertical_speed.mark_stale();
            self.vertical_speed.mark_stale();
            if matches!(state, DomainState::Unavailable) {
                self.estimator.reset_altitude();
                self.pressure_altitude = None;
            }
            return;
        };

        if self.pressure_altitude == Some(selected) {
            return;
        }
        if self
            .pressure_altitude
            .is_some_and(|previous| previous.source != selected.source)
        {
            self.estimator.reset_altitude();
            self.raw_vertical_speed.mark_stale();
            self.vertical_speed.mark_stale();
        }
        self.pressure_altitude = Some(selected);
        let SampleAcceptance::Accepted = self
            .estimator
            .pressure_altitude(selected.ingested_at.since_start(), selected.value)
        else {
            return;
        };
        let estimate = self.estimator.estimate();
        match estimate.raw_vertical_speed {
            Some(raw_vertical_speed) => self.raw_vertical_speed.update(raw_vertical_speed),
            None => self.raw_vertical_speed.mark_stale(),
        }
        match estimate.vertical_speed {
            Some(vertical_speed) => self.vertical_speed.update(vertical_speed),
            None => self.vertical_speed.mark_stale(),
        }
    }

    pub fn instruments(&self) -> Option<DerivedInstruments> {
        let (raw_vertical_speed, stale) = self.raw_vertical_speed.value_with_stale()?;
        let vertical_speed =
            self.vertical_speed
                .value_with_stale()
                .map(|(rate, stale)| SpeedInstrument {
                    meters_per_second: rate.as_meters_per_second(),
                    stale,
                });
        Some(DerivedInstruments {
            raw_vertical_speed: Some(SpeedInstrument {
                meters_per_second: raw_vertical_speed.as_meters_per_second(),
                stale,
            }),
            vertical_speed,
        })
    }
}

use super::estimator::Estimator;
use super::sample::SampleAcceptance;
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
    vario: SignalState<Speed>,
}

impl SensorFusion {
    /// Applies one coherent set of selected sensor states.
    pub fn update(&mut self, inputs: FusionInputs) {
        self.update_true_airspeed(inputs.true_airspeed);
        self.update_pressure_altitude(inputs.pressure_altitude);
    }

    fn update_true_airspeed(&mut self, state: DomainState<Speed>) {
        let DomainState::Current(selected) = state else {
            self.estimator.clear_air_speed();
            self.air_speed = None;
            self.vario.mark_stale();
            return;
        };
        if self.air_speed == Some(selected) {
            return;
        }
        if self
            .air_speed
            .is_some_and(|previous| previous.source != selected.source)
        {
            self.estimator.reset_air_speed();
            self.vario.mark_stale();
        }
        let SampleAcceptance::Accepted = self
            .estimator
            .air_speed(selected.ingested_at.since_start(), selected.value)
        else {
            return;
        };
        self.air_speed = Some(selected);
    }

    fn update_pressure_altitude(&mut self, state: DomainState<PressureAltitude>) {
        let DomainState::Current(selected) = state else {
            self.raw_vertical_speed.mark_stale();
            self.vertical_speed.mark_stale();
            self.vario.mark_stale();
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
            self.vario.mark_stale();
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
        if let Some(vario) = estimate.vario {
            self.vario.update(vario);
        } else {
            self.vario.mark_stale();
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
        let vario = self
            .vario
            .value_with_stale()
            .map(|(vertical_speed, stale)| SpeedInstrument {
                meters_per_second: vertical_speed.as_meters_per_second(),
                stale,
            });
        Some(DerivedInstruments {
            raw_vertical_speed: Some(SpeedInstrument {
                meters_per_second: raw_vertical_speed.as_meters_per_second(),
                stale,
            }),
            vertical_speed,
            vario,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExternalDeviceId;
    use crate::ownship::SourceId;
    use crate::time::Timestamp;
    use claims::assert_some;
    use updraft_units::Length;

    fn selected_from<T>(source: SourceId, value: T, milliseconds: u64) -> Selected<T> {
        Selected {
            source,
            ingested_at: Timestamp::from_millis(milliseconds),
            value,
        }
    }

    fn selected<T>(value: T, milliseconds: u64) -> Selected<T> {
        selected_from(SourceId::InternalGps, value, milliseconds)
    }

    fn inputs(
        true_airspeed: DomainState<Speed>,
        pressure_altitude: DomainState<PressureAltitude>,
    ) -> FusionInputs {
        FusionInputs {
            true_airspeed,
            pressure_altitude,
        }
    }

    #[test]
    fn stale_airspeed_does_not_stale_the_vertical_speeds() {
        let mut fusion = SensorFusion::default();
        let altitude = PressureAltitude::new(Length::from_meters(1_000.));
        let speed = selected(Speed::from_kilometers_per_hour(120.), 0);
        let current_speed = DomainState::Current(speed);
        let first_altitude = DomainState::Current(selected(altitude, 0));
        fusion.update(inputs(current_speed, first_altitude));
        let second_altitude = DomainState::Current(selected(altitude, 1_000));
        fusion.update(inputs(current_speed, second_altitude));

        let current = assert_some!(fusion.instruments());
        assert!(!assert_some!(current.raw_vertical_speed).stale);
        assert!(!assert_some!(current.vertical_speed).stale);
        assert!(!assert_some!(current.vario).stale);

        let stale_speed = DomainState::LastKnown(speed);
        fusion.update(inputs(stale_speed, second_altitude));

        let stale = assert_some!(fusion.instruments());
        assert!(!assert_some!(stale.raw_vertical_speed).stale);
        assert!(!assert_some!(stale.vertical_speed).stale);
        assert!(assert_some!(stale.vario).stale);
    }

    #[test]
    fn airspeed_source_change_restarts_the_vario_series() {
        let mut fusion = SensorFusion::default();
        let altitude = PressureAltitude::new(Length::from_meters(1_000.));
        let first = SourceId::InternalGps;
        let second = SourceId::External(ExternalDeviceId(1));
        let first_speed = selected_from(first, Speed::from_meters_per_second(50.), 0);
        let first_speed = DomainState::Current(first_speed);
        let first_altitude = DomainState::Current(selected(altitude, 0));
        fusion.update(inputs(first_speed, first_altitude));
        let second_altitude = DomainState::Current(selected(altitude, 1_000));
        fusion.update(inputs(first_speed, second_altitude));

        let second_speed = selected_from(second, Speed::from_meters_per_second(100.), 2_000);
        let second_speed = DomainState::Current(second_speed);
        let third_altitude = DomainState::Current(selected(altitude, 2_000));
        fusion.update(inputs(second_speed, third_altitude));

        let stale = assert_some!(assert_some!(fusion.instruments()).vario);
        assert_eq!(stale.meters_per_second, 0.);
        assert!(stale.stale);

        let fourth_altitude = DomainState::Current(selected(altitude, 3_000));
        fusion.update(inputs(second_speed, fourth_altitude));

        let current = assert_some!(assert_some!(fusion.instruments()).vario);
        assert_eq!(current.meters_per_second, 0.);
        assert!(!current.stale);
    }
}

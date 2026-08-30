use super::estimator::{Estimator, Fix, FixAcceptance};
use super::sample::SampleAcceptance;
use super::wind::Wind;
use crate::ownship::{DomainState, GpsSnapshot, Selected, SourceId};
use crate::signal_state::SignalState;
use crate::topic::{
    DerivedAltitudeInstruments, DerivedHeadingInstruments, DerivedInstruments,
    DerivedWindInstruments, SpeedInstrument,
};
use updraft_units::{Angle, MslAltitude, PressureAltitude, Speed};

/// Selected sensor states for one core time advancement.
///
/// The complete input keeps estimator updates independent of source-selection
/// call order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FusionInputs {
    pub gps: DomainState<GpsSnapshot>,
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
    air_speed_source: Option<SourceId>,
    gps: Option<Selected<GpsSnapshot>>,
    gps_altitude_source: Option<SourceId>,
    pressure_altitude: Option<Selected<PressureAltitude>>,
    air_speed_current: bool,
    gps_altitude_current: bool,
    pressure_altitude_current: bool,
    raw_vertical_speed: SignalState<Speed>,
    vertical_speed: SignalState<Speed>,
    vario: SignalState<Speed>,
    wind: SignalState<Wind>,
    derived_air_speed: SignalState<Speed>,
    heading: SignalState<Angle>,
    altitude: SignalState<MslAltitude>,
}

impl SensorFusion {
    /// Applies one coherent set of selected sensor states.
    pub fn update(&mut self, inputs: FusionInputs) {
        let gps_current = matches!(inputs.gps, DomainState::Current(_));
        let pressure_current = matches!(inputs.pressure_altitude, DomainState::Current(_));
        let gps_discontinuity = match inputs.gps {
            DomainState::Unavailable => self.gps.is_some(),
            DomainState::Current(selected) => self
                .gps
                .is_some_and(|previous| previous.source != selected.source),
            DomainState::LastKnown(_) => false,
        };
        let gps_altitude_discontinuity = match (self.gps_altitude_source, inputs.gps) {
            (Some(_), DomainState::Unavailable) => true,
            (Some(source), DomainState::Current(selected)) => source != selected.source,
            _ => false,
        };
        let pressure_discontinuity = match inputs.pressure_altitude {
            DomainState::Unavailable => self.pressure_altitude.is_some(),
            DomainState::Current(selected) => self
                .pressure_altitude
                .is_some_and(|previous| previous.source != selected.source),
            DomainState::LastKnown(_) => false,
        };
        let air_speed_current = matches!(inputs.true_airspeed, DomainState::Current(_));
        if pressure_discontinuity || (gps_altitude_discontinuity && !pressure_current) {
            self.estimator.reset_altitude();
            self.gps = None;
            self.gps_altitude_source = None;
            self.pressure_altitude = None;
            self.mark_altitude_estimates_stale();
        } else if gps_altitude_discontinuity {
            self.estimator.reset_gnss_altitude();
            self.gps = None;
            self.gps_altitude_source = None;
            self.altitude.mark_stale();
        }
        if gps_discontinuity {
            self.estimator.reset_wind();
            self.wind.mark_stale();
            self.heading.mark_stale();
            if !air_speed_current {
                self.derived_air_speed.mark_stale();
            }
        }

        let gps_altitude_current = match inputs.gps {
            DomainState::Current(selected) => selected.value.altitude_msl.is_some(),
            _ => false,
        };
        self.air_speed_current = air_speed_current;
        self.gps_altitude_current = gps_altitude_current;
        self.pressure_altitude_current = pressure_current;
        self.update_true_airspeed(inputs.true_airspeed);
        if !gps_current {
            self.update_gps(inputs.gps);
        }
        if !pressure_current {
            self.update_pressure_altitude(inputs.pressure_altitude);
        }
        if gps_current {
            self.update_gps(inputs.gps);
        }
        if pressure_current {
            self.update_pressure_altitude(inputs.pressure_altitude);
        }
    }

    fn update_true_airspeed(&mut self, state: DomainState<Speed>) {
        let DomainState::Current(selected) = state else {
            self.estimator.clear_air_speed();
            self.vario.mark_stale();
            self.air_speed_current = false;
            self.derived_air_speed.mark_stale();
            self.air_speed = None;
            return;
        };
        if self.air_speed == Some(selected) {
            return;
        }
        if self
            .air_speed_source
            .is_some_and(|source| source != selected.source)
        {
            self.estimator.reset_air_speed();
            self.vario.mark_stale();
            self.estimator.reset_wind();
            self.wind.mark_stale();
            self.heading.mark_stale();
        }
        let SampleAcceptance::Accepted = self
            .estimator
            .air_speed(selected.ingested_at.since_start(), selected.value)
        else {
            return;
        };
        self.air_speed = Some(selected);
        self.air_speed_source = Some(selected.source);
        self.derived_air_speed.update(selected.value);
    }

    fn update_gps(&mut self, state: DomainState<GpsSnapshot>) {
        let DomainState::Current(selected) = state else {
            self.estimator.clear_inferred_air_speed();
            self.wind.mark_stale();
            self.heading.mark_stale();
            if !self.air_speed_current {
                self.derived_air_speed.mark_stale();
            }
            if !self.pressure_altitude_current {
                self.mark_altitude_estimates_stale();
            }
            if matches!(state, DomainState::Unavailable) {
                self.gps = None;
            }
            return;
        };

        if self.gps == Some(selected) {
            return;
        }
        self.gps = Some(selected);
        self.estimator.position(selected.value.position);
        self.update_ground_velocity(selected.value);

        if let Some(altitude) = selected.value.altitude_msl {
            let ellipsoid =
                updraft_egm96::msl_to_ellipsoidal(selected.value.position, altitude.value);
            if self
                .estimator
                .gnss_altitude(altitude.ingested_at.since_start(), ellipsoid)
                == SampleAcceptance::Accepted
            {
                self.gps_altitude_source = Some(selected.source);
                self.update_altitude_estimate();
            }
        } else if !self.pressure_altitude_current {
            self.mark_altitude_estimates_stale();
        }
    }

    fn update_ground_velocity(&mut self, gps: GpsSnapshot) {
        let Some((track, ground_speed)) = gps
            .track
            .zip(gps.ground_speed)
            .filter(|(track, ground_speed)| track.ingested_at == ground_speed.ingested_at)
        else {
            self.estimator.clear_inferred_air_speed();
            self.wind.mark_stale();
            self.heading.mark_stale();
            if !self.air_speed_current {
                self.derived_air_speed.mark_stale();
            }
            return;
        };
        let acceptance = self.estimator.fix(
            track.ingested_at.since_start(),
            &Fix {
                track: track.value,
                ground_speed: ground_speed.value,
            },
        );
        let refresh_motion_estimate = acceptance != FixAcceptance::Ignored
            && (acceptance != FixAcceptance::Predicted
                || !matches!(self.wind, SignalState::LastKnown(_)));
        if refresh_motion_estimate {
            self.update_motion_estimate();
        }
        if acceptance == FixAcceptance::RejectedWindMeasurement {
            self.wind.mark_stale();
            self.heading.mark_stale();
        }
    }

    fn update_pressure_altitude(&mut self, state: DomainState<PressureAltitude>) {
        let DomainState::Current(selected) = state else {
            self.estimator.clear_pressure_altitude();
            if !self.gps_altitude_current {
                self.mark_altitude_estimates_stale();
            }
            if matches!(state, DomainState::Unavailable) {
                self.pressure_altitude = None;
            }
            return;
        };

        if self.pressure_altitude == Some(selected) {
            return;
        }
        self.pressure_altitude = Some(selected);
        let SampleAcceptance::Accepted = self
            .estimator
            .pressure_altitude(selected.ingested_at.since_start(), selected.value)
        else {
            return;
        };
        self.update_altitude_estimate();
    }

    fn mark_altitude_estimates_stale(&mut self) {
        self.raw_vertical_speed.mark_stale();
        self.vertical_speed.mark_stale();
        self.vario.mark_stale();
        self.altitude.mark_stale();
    }

    fn update_altitude_estimate(&mut self) {
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
        if let Some(altitude) = estimate.altitude {
            self.altitude.update(altitude);
        }
    }

    fn update_motion_estimate(&mut self) {
        let estimate = self.estimator.estimate();
        if let Some(wind) = estimate.wind {
            self.wind.update(wind);
        } else {
            self.wind.mark_stale();
        }
        if let Some(air_speed) = estimate.air_speed {
            self.derived_air_speed.update(air_speed);
        } else {
            self.derived_air_speed.mark_stale();
        }
        if let Some(heading) = estimate.heading {
            self.heading.update(heading);
        } else {
            self.heading.mark_stale();
        }
    }

    pub fn instruments(&self) -> Option<DerivedInstruments> {
        let raw_vertical_speed = self
            .raw_vertical_speed
            .value_with_stale()
            .map(|(rate, stale)| SpeedInstrument {
                meters_per_second: rate.as_meters_per_second(),
                stale,
            });
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
        let wind = self
            .wind
            .value_with_stale()
            .map(|(wind, stale)| DerivedWindInstruments {
                direction_degrees: wind.direction.as_degrees(),
                speed_meters_per_second: wind.speed.as_meters_per_second(),
                stale,
            });
        let airspeed = self
            .derived_air_speed
            .value_with_stale()
            .map(|(speed, stale)| SpeedInstrument {
                meters_per_second: speed.as_meters_per_second(),
                stale,
            });
        let heading =
            self.heading
                .value_with_stale()
                .map(|(heading, stale)| DerivedHeadingInstruments {
                    degrees: heading.as_degrees(),
                    stale,
                });
        let altitude =
            self.altitude
                .value_with_stale()
                .map(|(altitude, stale)| DerivedAltitudeInstruments {
                    altitude_msl_meters: altitude.into_inner().as_meters(),
                    stale,
                });

        let available = raw_vertical_speed.is_some()
            || vertical_speed.is_some()
            || vario.is_some()
            || wind.is_some()
            || airspeed.is_some()
            || heading.is_some()
            || altitude.is_some();
        available.then_some(DerivedInstruments {
            raw_vertical_speed,
            vertical_speed,
            vario,
            wind,
            airspeed,
            heading,
            altitude,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExternalDeviceId;
    use crate::ownship::SourceId;
    use crate::time::Timestamp;
    use claims::{assert_none, assert_some};
    use updraft_geo::LatLon;
    use updraft_units::{Length, MslAltitude};

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
            gps: DomainState::Unavailable,
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

    #[test]
    fn current_gnss_altitude_remains_fresh_without_pressure() {
        let mut fusion = SensorFusion::default();
        let gps = |meters, milliseconds| {
            let snapshot = GpsSnapshot {
                position: LatLon::from_degrees(50., 6.),
                altitude_msl: Some(crate::ownship::Timed::new(
                    MslAltitude::new(Length::from_meters(meters)),
                    Timestamp::from_millis(milliseconds),
                )),
                track: None,
                ground_speed: None,
                fix_time: None,
            };
            DomainState::Current(selected(snapshot, milliseconds))
        };
        let inputs = |gps| FusionInputs {
            gps,
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
        };

        fusion.update(inputs(gps(1_000., 0)));
        let current = gps(1_001., 1_000);
        fusion.update(inputs(current));
        fusion.update(inputs(current));

        let instruments = assert_some!(fusion.instruments());
        assert!(!assert_some!(instruments.raw_vertical_speed).stale);
        assert!(!assert_some!(instruments.vertical_speed).stale);
        assert!(!assert_some!(instruments.altitude).stale);
    }

    #[test]
    fn gps_source_change_to_position_only_preserves_pressure_vertical_speed() {
        let mut fusion = SensorFusion::default();
        let gps = |source, longitude, altitude: Option<f64>, milliseconds| {
            let snapshot = GpsSnapshot {
                position: LatLon::from_degrees(50., longitude),
                altitude_msl: altitude.map(|meters| {
                    crate::ownship::Timed::new(
                        MslAltitude::new(Length::from_meters(meters)),
                        Timestamp::from_millis(milliseconds),
                    )
                }),
                track: None,
                ground_speed: None,
                fix_time: None,
            };
            DomainState::Current(selected_from(source, snapshot, milliseconds))
        };
        let first_source = SourceId::External(ExternalDeviceId(1));
        let second_source = SourceId::External(ExternalDeviceId(2));
        let first_gps = gps(first_source, 6., Some(1_200.), 0);
        let altitude = PressureAltitude::new(Length::from_meters(1_000.));
        let first_pressure = DomainState::Current(selected(altitude, 0));
        let second_pressure = DomainState::Current(selected(altitude, 1_000));

        fusion.update(FusionInputs {
            gps: first_gps,
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: first_pressure,
        });
        fusion.update(FusionInputs {
            gps: gps(first_source, 6., Some(1_200.), 1_000),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: second_pressure,
        });
        fusion.update(FusionInputs {
            gps: gps(second_source, 7., None, 2_000),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: second_pressure,
        });

        let instruments = assert_some!(fusion.instruments());
        assert!(!assert_some!(instruments.raw_vertical_speed).stale);
        assert!(!assert_some!(instruments.vertical_speed).stale);

        fusion.update(FusionInputs {
            gps: gps(second_source, 7., None, 2_000),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Current(selected(altitude, 2_000)),
        });
        let instruments = assert_some!(fusion.instruments());
        assert_eq!(
            assert_some!(instruments.raw_vertical_speed).meters_per_second,
            0.
        );
        assert!(assert_some!(instruments.altitude).stale);
    }

    #[test]
    fn unavailable_gps_preserves_pressure_vertical_speed() {
        let mut fusion = SensorFusion::default();
        let altitude = PressureAltitude::new(Length::from_meters(1_000.));
        let pressure = |milliseconds| DomainState::Current(selected(altitude, milliseconds));
        let gps = |milliseconds| {
            DomainState::Current(selected(
                GpsSnapshot {
                    position: LatLon::from_degrees(50., 6.),
                    altitude_msl: Some(crate::ownship::Timed::new(
                        MslAltitude::new(Length::from_meters(1_200.)),
                        Timestamp::from_millis(milliseconds),
                    )),
                    track: None,
                    ground_speed: None,
                    fix_time: None,
                },
                milliseconds,
            ))
        };

        fusion.update(FusionInputs {
            gps: gps(0),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: pressure(0),
        });
        let current_pressure = pressure(1_000);
        fusion.update(FusionInputs {
            gps: gps(1_000),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: current_pressure,
        });
        fusion.update(FusionInputs {
            gps: DomainState::Unavailable,
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: current_pressure,
        });

        let instruments = assert_some!(fusion.instruments());
        assert!(!assert_some!(instruments.raw_vertical_speed).stale);
        assert!(!assert_some!(instruments.vertical_speed).stale);

        fusion.update(FusionInputs {
            gps: DomainState::Unavailable,
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: pressure(2_000),
        });
        let instruments = assert_some!(fusion.instruments());
        assert_eq!(
            assert_some!(instruments.raw_vertical_speed).meters_per_second,
            0.
        );
        assert!(assert_some!(instruments.altitude).stale);
    }

    #[test]
    fn position_refresh_preserves_gnss_altitude_without_repeating_its_sample() {
        let mut fusion = SensorFusion::default();
        let snapshot = |longitude, milliseconds| {
            selected(
                GpsSnapshot {
                    position: LatLon::from_degrees(50., longitude),
                    altitude_msl: Some(crate::ownship::Timed::new(
                        MslAltitude::new(Length::from_meters(1_000.)),
                        Timestamp::from_millis(0),
                    )),
                    track: None,
                    ground_speed: None,
                    fix_time: None,
                },
                milliseconds,
            )
        };

        fusion.update(FusionInputs {
            gps: DomainState::Current(snapshot(6., 0)),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
        });
        let instruments = assert_some!(fusion.instruments());
        assert_none!(instruments.raw_vertical_speed);
        assert!(!assert_some!(instruments.altitude).stale);

        fusion.update(FusionInputs {
            gps: DomainState::Current(snapshot(6.001, 1_000)),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
        });

        let instruments = assert_some!(fusion.instruments());
        assert_none!(instruments.raw_vertical_speed);
        assert!(!assert_some!(instruments.altitude).stale);
    }
}

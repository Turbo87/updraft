use time::{Time, Weekday};

/// One of the six operating schedules permitted by the OpenAIP schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirspaceOperatingSchedule {
    /// A fixed start time and fixed end time.
    Fixed { start_time: Time, end_time: Time },
    /// A fixed start time and a sunset end.
    FixedStartUntilSunset { start_time: Time },
    /// A sunrise start and a fixed end time.
    SunriseUntilFixedEnd { end_time: Time },
    /// A sunrise start and a sunset end.
    SunriseUntilSunset,
    /// No explicit time or sun marker.
    NoSpecifiedTime,
    /// Activation by NOTAM.
    ByNotam,
}

/// One day-specific airspace operating period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AirspaceOperatingPeriod {
    /// The day on which this period applies.
    pub day_of_week: Weekday,
    /// Whether this period excludes public holidays.
    pub public_holidays_excluded: bool,
    /// Additional source remarks for this period.
    pub remarks: Option<Box<str>>,
    /// The operating schedule for this period.
    pub schedule: AirspaceOperatingSchedule,
}

/// One or more day-specific airspace operating periods.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AirspaceOperatingHours {
    operating_periods: Box<[AirspaceOperatingPeriod]>,
    /// Additional source remarks for the operating hours.
    pub remarks: Option<Box<str>>,
}

impl AirspaceOperatingHours {
    /// Creates operating hours when at least one period is present.
    pub fn new(
        operating_periods: Vec<AirspaceOperatingPeriod>,
        remarks: Option<Box<str>>,
    ) -> Option<Self> {
        if operating_periods.is_empty() {
            return None;
        }

        Some(Self {
            operating_periods: operating_periods.into_boxed_slice(),
            remarks,
        })
    }

    /// Returns all operating periods.
    pub fn operating_periods(&self) -> &[AirspaceOperatingPeriod] {
        &self.operating_periods
    }
}

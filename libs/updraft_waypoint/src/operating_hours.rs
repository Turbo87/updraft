use time::{Time, Weekday};

/// One of the six operating schedules permitted by the OpenAIP schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatingSchedule {
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
    /// Operation by NOTAM.
    ByNotam,
}

/// One day-specific operating period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatingPeriod {
    /// The day on which this period applies.
    pub day_of_week: Weekday,
    /// Whether this period excludes public holidays.
    pub public_holidays_excluded: bool,
    /// Additional source remarks for this period.
    pub remarks: Option<Box<str>>,
    /// The operating schedule for this period.
    pub schedule: OperatingSchedule,
}

/// One or more day-specific operating periods.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatingHours {
    operating_periods: Box<[OperatingPeriod]>,
    /// Additional source remarks for the operating hours.
    pub remarks: Option<Box<str>>,
}

impl OperatingHours {
    /// Creates operating hours when at least one period is present.
    pub fn new(operating_periods: Vec<OperatingPeriod>, remarks: Option<Box<str>>) -> Option<Self> {
        if operating_periods.is_empty() {
            return None;
        }

        Some(Self {
            operating_periods: operating_periods.into_boxed_slice(),
            remarks,
        })
    }

    /// Returns all operating periods.
    pub fn operating_periods(&self) -> &[OperatingPeriod] {
        &self.operating_periods
    }
}

use claims::{assert_none, assert_some};
use time::{Time, Weekday};
use updraft_waypoint::{OperatingHours, OperatingPeriod, OperatingSchedule};

/// Verifies that operating hours need at least one period.
#[test]
fn rejects_operating_hours_without_a_period() {
    assert_none!(OperatingHours::new(Vec::new(), None));
}

/// Verifies that operating hours keep their periods in source order.
#[test]
fn keeps_operating_periods_in_source_order() {
    let periods = vec![
        period(Weekday::Monday, OperatingSchedule::SunriseUntilSunset),
        period(
            Weekday::Tuesday,
            OperatingSchedule::Fixed {
                start_time: Time::from_hms(8, 0, 0).unwrap(),
                end_time: Time::from_hms(18, 30, 0).unwrap(),
            },
        ),
    ];

    let hours = assert_some!(OperatingHours::new(periods.clone(), None));

    assert_eq!(hours.operating_periods(), periods);
}

fn period(day_of_week: Weekday, schedule: OperatingSchedule) -> OperatingPeriod {
    OperatingPeriod {
        day_of_week,
        public_holidays_excluded: false,
        remarks: None,
        schedule,
    }
}

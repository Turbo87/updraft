//! Task detection cost over a synthetic task for a recorded flight.

use criterion::{Criterion, criterion_group, criterion_main};
use igc::records::Record;
use std::hint::black_box;
use updraft_core::{
    ChangeTask, Core, Fix, GetTask, InternalGps, NavigationTarget, SettingsSnapshot, TaskCommand,
    Timestamp, UtcInstant,
};
use updraft_geo::LatLon;

const RECORDING: &str = include_str!("../../../testdata/weglide_1141558.igc");
/// 2026-07-04, from the `HFDTE` record.
const FLIGHT_DATE: UtcInstant = UtcInstant::from_unix_milliseconds(1_783_123_200_000);
/// The flight passes through each 500 m cylinder. The start and finish point
/// is 1.6 km from Stolberg-Diepenlinchen, and the first turnpoint is 1.0 km
/// from Utscheid. The second turnpoint is the most eastern fix.
const TURNPOINTS: [(&str, f64, f64); 4] = [
    ("Stolberg", 50.7685, 6.3045),
    ("Utscheid", 49.9904, 6.3512),
    ("East", 50.4664, 8.4747),
    ("Stolberg", 50.7685, 6.3045),
];

fn fixes() -> Vec<(Timestamp, Fix)> {
    let mut fixes = Vec::new();
    for line in RECORDING.lines() {
        if let Ok(Record::B(record)) = Record::parse_line(line) {
            let seconds = record.timestamp.seconds_since_midnight();
            let at = Timestamp::from_millis(u64::from(seconds) * 1000);
            let fix = Fix {
                position: LatLon::from_degrees(record.pos.lat.into(), record.pos.lon.into()),
                altitude_ellipsoid: None,
                track: None,
                ground_speed: None,
                fix_time: Some(FLIGHT_DATE.saturating_add(at.since_start())),
            };
            fixes.push((at, fix));
        }
    }
    fixes
}

fn replay(fixes: &[(Timestamp, Fix)], task: bool) -> Core {
    let mut core = Core::new(SettingsSnapshot::default());
    let start = Timestamp::default();
    if task {
        for (name, latitude_degrees, longitude_degrees) in TURNPOINTS {
            let target = NavigationTarget::Waypoint {
                name: name.into(),
                latitude_degrees,
                longitude_degrees,
                elevation_meters: 0.,
            };
            let add = ChangeTask(TaskCommand::Add { target });
            core.apply(add, start).response.unwrap();
        }
        let select = ChangeTask(TaskCommand::Select { id: 0 });
        core.apply(select, start).response.unwrap();
    }
    for &(at, fix) in fixes {
        black_box(core.apply(InternalGps::new(fix), at));
    }
    core
}

fn bench_task(c: &mut Criterion) {
    let fixes = fixes();
    let mut core = replay(&fixes, true);
    let task = core.apply(GetTask, Timestamp::default()).response;
    claims::assert_some!(task.progress.finish);
    let mut group = c.benchmark_group("weglide_1141558");
    group.sample_size(10);
    group.bench_function("without_task", |b| b.iter(|| replay(&fixes, false)));
    group.bench_function("with_task", |b| b.iter(|| replay(&fixes, true)));
    group.finish();
}

criterion_group!(benches, bench_task);
criterion_main!(benches);

use crate::replay::Replay;
use claims::assert_ok;
use updraft_core::{
    Bytes, Core, ExternalDeviceConfig, ExternalDeviceId, SettingsSnapshot, Timestamp, Topic,
    TrafficUpdate,
};

#[test]
fn replay_preserves_flarm_cycle_references_across_batches_and_fragments() {
    let fix = "$GPRMC,120000,A,5000,N,00800,E,100,90,050826,,,A\r\n";
    let cycle = "$PFLAU,1,1,2,1,0,,0,,,\r\n$PGRMZ,1000,f,3\r\n";
    let target = "$PFLAA,0,0,0,0,1,ABC123,90,0,40,0,1,0,0\r\n";
    let next = "$GPRMC,120001,A,5000,N,00800.1,E,100,0,050826,,,A\r\n";
    let altitude = "$GPGGA,120000,5000,N,00800,E,1,08,1,100,M,0,M,,\r\n";
    let next_altitude = "$GPGGA,120001,5000,N,00800,E,1,08,1,103,M,0,M,,\r\n";
    let recording = [
        fix,
        altitude,
        cycle,
        target,
        next,
        next_altitude,
        target,
        cycle,
        target,
    ]
    .concat();
    let replay = assert_ok!(Replay::from_nmea(recording.as_bytes().to_vec()));
    assert_eq!(replay.events().len(), 2);
    assert_eq!(replay.events()[1].at().as_millis(), 1_000);
    let payload: Vec<u8> = replay
        .events()
        .iter()
        .flat_map(|event| event.payload().iter().copied())
        .collect();
    assert_eq!(payload, recording.as_bytes());
    let run = |chunk_size, time_scale| {
        let mut core = Core::new(SettingsSnapshot {
            external_devices: vec![ExternalDeviceConfig {
                enabled: true,
                spec: updraft_core::ConnectionSpec::tcp("localhost", 4353),
            }],
            ..SettingsSnapshot::default()
        });
        let mut positions = Vec::new();
        for event in replay.events() {
            let at = Timestamp::from_millis(event.at().as_millis() as u64 / time_scale);
            for chunk in event.payload().chunks(chunk_size) {
                core.apply(Bytes::new(ExternalDeviceId(1), chunk), at);
            }
            let target = core
                .topics()
                .into_iter()
                .find_map(|topic| match topic {
                    Topic::Traffic(TrafficUpdate::Snapshot(mut targets)) => Some(targets.remove(0)),
                    _ => None,
                })
                .expect("the replay produces traffic");
            positions.push((target.position, target.altitude_msl_meters));
        }
        positions
    };
    let batched = run(usize::MAX, 1);
    assert_ne!(batched[0].0, batched[1].0);
    claims::assert_some_eq!(batched[0].1, 100.);
    claims::assert_some_eq!(batched[1].1, 109.);
    assert_eq!(run(1, 1), batched);
    assert_eq!(run(7, 10), batched);
    let origin = updraft_geo::LatLon::from_degrees(50., 8.);
    let first = updraft_geo::LatLon::from_degrees(
        batched[0].0.latitude_degrees,
        batched[0].0.longitude_degrees,
    );
    approx::assert_abs_diff_eq!(
        origin.distance(first).as_meters(),
        102.8888888889,
        epsilon = 1e-6
    );
}

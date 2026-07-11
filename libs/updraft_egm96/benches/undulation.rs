//! Throughput benchmark for the geoid undulation lookup over a sweep of
//! positions spanning the globe.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use updraft_egm96::undulation;
use updraft_geo::LatLon;

/// A deterministic sweep of query positions covering the whole grid: every
/// combination of a set of latitudes (pole to pole) and longitudes (across
/// the antimeridian), offset off the integer nodes so each lookup actually
/// bilinearly interpolates rather than hitting a grid node directly.
fn sweep() -> Vec<LatLon> {
    let mut positions = Vec::new();
    let mut lat = -89.5;
    while lat <= 89.5 {
        let mut lon = -179.5;
        while lon <= 179.5 {
            positions.push(LatLon::from_degrees(lat, lon));
            lon += 7.3;
        }
        lat += 4.7;
    }
    positions
}

/// Runs the lookup over every position in the sweep.
fn lookup_all(positions: &[LatLon]) {
    for &position in positions {
        black_box(undulation(black_box(position)));
    }
}

fn bench_undulation(c: &mut Criterion) {
    let positions = sweep();

    let mut group = c.benchmark_group("undulation");

    group.throughput(Throughput::Elements(positions.len() as u64));
    group.bench_function("sweep", |b| {
        b.iter(|| lookup_all(black_box(&positions)));
    });

    group.throughput(Throughput::Elements(1));
    group.bench_function("single", |b| {
        let position = LatLon::from_degrees(52.0, 7.5);
        b.iter(|| black_box(undulation(black_box(position))));
    });

    group.finish();
}

criterion_group!(benches, bench_undulation);
criterion_main!(benches);

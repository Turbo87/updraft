//! Compares the cheap ruler with the geodesic and haversine methods of
//! `LatLon`.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use updraft_geo::{CheapRuler, LatLon};
use updraft_units::{Angle, Length};

const DISTANCES_KILOMETERS: [f64; 4] = [1., 10., 100., 500.];

fn origin() -> LatLon {
    LatLon::from_degrees(50., 10.)
}

fn bench_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance");
    let origin = origin();
    let ruler = CheapRuler::new(origin.latitude());

    for kilometers in DISTANCES_KILOMETERS {
        let target = origin.destination(
            Angle::from_degrees(45.),
            Length::from_kilometers(kilometers),
        );
        let id = format!("{kilometers} km");

        group.bench_with_input(BenchmarkId::new("ruler", &id), &target, |b, &target| {
            b.iter(|| ruler.distance(black_box(origin), black_box(target)));
        });
        // A ruler at the mean latitude of each pair, the setup with the
        // documented accuracy.
        group.bench_with_input(
            BenchmarkId::new("ruler_per_pair", &id),
            &target,
            |b, &target| {
                b.iter(|| {
                    let (a, b) = (black_box(origin), black_box(target));
                    CheapRuler::new((a.latitude() + b.latitude()) / 2.).distance(a, b)
                });
            },
        );
        group.bench_with_input(BenchmarkId::new("geodesic", &id), &target, |b, &target| {
            b.iter(|| black_box(origin).distance(black_box(target)));
        });
        group.bench_with_input(BenchmarkId::new("haversine", &id), &target, |b, &target| {
            b.iter(|| black_box(origin).haversine_distance(black_box(target)));
        });
    }

    group.finish();
}

fn bench_destination(c: &mut Criterion) {
    let mut group = c.benchmark_group("destination");
    let origin = origin();
    let ruler = CheapRuler::new(origin.latitude());
    let bearing = Angle::from_degrees(45.);

    for kilometers in DISTANCES_KILOMETERS {
        let distance = Length::from_kilometers(kilometers);
        let id = format!("{kilometers} km");

        group.bench_with_input(BenchmarkId::new("ruler", &id), &distance, |b, &distance| {
            b.iter(|| {
                ruler.destination(black_box(origin), black_box(bearing), black_box(distance))
            });
        });
        group.bench_with_input(
            BenchmarkId::new("geodesic", &id),
            &distance,
            |b, &distance| {
                b.iter(|| black_box(origin).destination(black_box(bearing), black_box(distance)));
            },
        );
    }

    group.finish();
}

fn bench_line(c: &mut Criterion) {
    // A circle with a radius of 10 km, similar to an `OpenAir` circle.
    const VERTICES: u32 = 100;
    let center = origin();
    let ring: Vec<_> = (0..=VERTICES)
        .map(|i| {
            let bearing = Angle::from_degrees(f64::from(i) * 360. / f64::from(VERTICES));
            center.destination(bearing, Length::from_kilometers(10.))
        })
        .collect();
    let ruler = CheapRuler::new(center.latitude());
    let point = center.destination(Angle::from_degrees(30.), Length::from_kilometers(12.));

    let mut group = c.benchmark_group("line");
    group.throughput(Throughput::Elements(ring.len() as u64));
    group.bench_function("line_distance", |b| {
        b.iter(|| ruler.line_distance(black_box(&ring)));
    });
    group.bench_function("point_on_line", |b| {
        b.iter(|| ruler.point_on_line(black_box(&ring), black_box(point)));
    });
    group.finish();
}

criterion_group!(benches, bench_distance, bench_destination, bench_line);
criterion_main!(benches);

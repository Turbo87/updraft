use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use std::{f64::consts::PI, hint::black_box, path::Path};
use updraft_geo::LatLon;
use updraft_terrain::TerrainReader;

fn terrain_reader() -> TerrainReader {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/terrain/france.terrain");
    let mut terrain = TerrainReader::default();
    terrain.insert("france".into(), &path).unwrap();
    terrain
}

fn flight_positions() -> Vec<LatLon> {
    include_str!("../../../testdata/terrain/flight.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let mut fields = line.split(',').skip(1);
            let latitude = fields.next().unwrap().parse().unwrap();
            let longitude = fields.next().unwrap().parse().unwrap();
            LatLon::from_degrees(latitude, longitude)
        })
        .collect()
}

fn tile_position(x: f64, y: f64) -> LatLon {
    let latitude = (PI * (1.0 - 2.0 * y / 1024.0)).sinh().atan().to_degrees();
    let longitude = x / 1024.0 * 360.0 - 180.0;
    LatLon::from_degrees(latitude, longitude)
}

fn sample(terrain: &TerrainReader, position: LatLon) {
    let elevation = terrain.elevation(black_box(position)).unwrap().unwrap();
    black_box(elevation);
}

fn sample_positions(terrain: &TerrainReader, positions: &[LatLon]) {
    for &position in black_box(positions) {
        sample(terrain, position);
    }
}

fn bench_elevation(c: &mut Criterion) {
    let terrain = terrain_reader();
    let interior = [
        tile_position(528.25, 371.25),
        tile_position(528.75, 371.25),
        tile_position(528.75, 371.75),
        tile_position(528.25, 371.75),
    ];
    sample_positions(&terrain, &interior);
    let mut group = c.benchmark_group("terrain_lookup");
    group.throughput(Throughput::Elements(interior.len() as u64));
    group.bench_function("warm/interior", |b| {
        b.iter(|| sample_positions(&terrain, &interior));
    });

    group.throughput(Throughput::Elements(1));
    group.bench_function("decoded_cache_miss/interior", |b| {
        b.iter_batched_ref(
            terrain_reader,
            |terrain| sample(terrain, interior[0]),
            BatchSize::PerIteration,
        );
    });

    let edge_inset = 0.25 / 256.0;
    let coverage_edge = tile_position(527.0 + edge_inset, 371.5);
    let coverage_corner = tile_position(527.0 + edge_inset, 370.0 + edge_inset);
    for (name, position) in [
        ("warm/edge", tile_position(528.0, 371.5)),
        ("warm/corner", tile_position(528.0, 371.0)),
        ("warm/coverage_edge", coverage_edge),
        ("warm/coverage_corner", coverage_corner),
    ] {
        sample(&terrain, position);
        group.bench_function(name, |b| b.iter(|| sample(&terrain, position)));
    }

    let positions = flight_positions();
    assert_eq!(positions.len(), 225);
    sample_positions(&terrain, &positions);
    group.throughput(Throughput::Elements(positions.len() as u64));
    group.bench_function("warm/flight_segment", |b| {
        b.iter(|| sample_positions(&terrain, &positions));
    });
    group.finish();
}

criterion_group!(benches, bench_elevation);
criterion_main!(benches);

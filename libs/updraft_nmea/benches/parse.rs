//! Throughput benchmark for the framing parser over a real flight recording.
//!
//! The fixture is a ~217 KiB capture of a full flight, mixing GNSS
//! (`GGA`/`RMC`/`GSA`), Garmin (`PGRMZ`), and FLARM (`PFLAU`/`PFLAA`)
//! sentences. Two shapes are measured: draining the whole buffer in one
//! pass, and feeding it in fixed-size chunks the way a live transport
//! delivers bytes, which exercises the resynchronisation and buffer-drain
//! paths as well as the happy path.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use updraft_nmea::{Step, parse};

const FLIGHT: &[u8] = include_bytes!("../../../testdata/flight_1.nmea");

/// Drains every sentence out of a complete byte buffer, stopping at the
/// first `Incomplete`.
fn parse_all(mut input: &[u8]) -> usize {
    let mut frames = 0;
    loop {
        match parse(&mut input) {
            Step::Incomplete => return frames,
            step => {
                black_box(&step);
                frames += 1;
            }
        }
    }
}

/// Feeds the bytes in fixed-size chunks through a growing buffer, the way
/// a live transport delivers them, draining whatever completes after each
/// chunk.
fn parse_streamed(bytes: &[u8], chunk: usize) -> usize {
    let mut buffer: Vec<u8> = Vec::new();
    let mut frames = 0;
    for slice in bytes.chunks(chunk) {
        buffer.extend_from_slice(slice);
        loop {
            let mut cursor = buffer.as_slice();
            match parse(&mut cursor) {
                Step::Incomplete => {
                    let consumed = buffer.len() - cursor.len();
                    buffer.drain(..consumed);
                    break;
                }
                step => {
                    black_box(&step);
                    let consumed = buffer.len() - cursor.len();
                    frames += 1;
                    buffer.drain(..consumed);
                }
            }
        }
    }
    frames
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("flight_1");
    group.throughput(Throughput::Bytes(FLIGHT.len() as u64));

    group.bench_function("parse_all", |b| {
        b.iter(|| parse_all(black_box(FLIGHT)));
    });

    for chunk in [64, 512] {
        group.bench_with_input(
            format!("parse_streamed/{chunk}"),
            &chunk,
            |b, &chunk| b.iter(|| parse_streamed(black_box(FLIGHT), chunk)),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);

use updraft_nmea::{Message, RejectReason, Step, parse};

const FLIGHT: &[u8] = include_bytes!("../../../testdata/flight_1.nmea");

/// Drains every sentence out of a complete byte buffer, stopping at the
/// first `Incomplete`.
fn parse_all(mut input: &[u8]) -> Vec<Step> {
    let mut steps = Vec::new();
    loop {
        match parse(&mut input) {
            Step::Incomplete => return steps,
            step => steps.push(step),
        }
    }
}

/// Feeds the bytes in fixed-size chunks through a growing buffer, the way
/// a live transport delivers them, draining whatever completes after each
/// chunk.
fn parse_streamed(bytes: &[u8], chunk: usize) -> Vec<Step> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut steps = Vec::new();
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
                    let consumed = buffer.len() - cursor.len();
                    steps.push(step);
                    buffer.drain(..consumed);
                }
            }
        }
    }
    steps
}

#[test]
fn parses_every_sentence_in_the_flight() {
    let steps = parse_all(FLIGHT);

    let mut gga = 0;
    let mut rmc = 0;
    let mut gsa = 0;
    let mut pgrmz = 0;
    let mut unknown = 0;
    let mut junk = 0;
    for step in &steps {
        match step {
            Step::Frame(Message::Gga(_)) => gga += 1,
            Step::Frame(Message::Rmc(_)) => rmc += 1,
            Step::Frame(Message::Gsa(_)) => gsa += 1,
            Step::Frame(Message::Pgrmz(_)) => pgrmz += 1,
            Step::Frame(Message::Unknown(_)) => unknown += 1,
            Step::Rejected(RejectReason::Junk) => junk += 1,
            other => panic!("unexpected step: {other:?}"),
        }
    }

    // Every line's sentence is recovered, including the two that carry
    // trailing bytes after a valid checksum.
    assert_eq!(gga, 466);
    assert_eq!(rmc, 466);
    assert_eq!(gsa, 469);
    assert_eq!(pgrmz, 467);
    // The FLARM sentences ($PFLAA, $PFLAU) are not decoded here yet.
    assert_eq!(unknown, 2377);
    // The trailing bytes on those two lines are rejected as junk.
    assert_eq!(junk, 2);
    assert_eq!(steps.len(), 4247);
}

#[test]
fn chunk_boundaries_do_not_change_the_frames() {
    // A live transport can split the stream anywhere. Framing must recover
    // the identical sentences regardless of chunk size. (Rejected junk can
    // coalesce differently across chunk boundaries, so compare the frames.)

    let frames = |steps: Vec<Step>| {
        steps
            .into_iter()
            .filter(|step| matches!(step, Step::Frame(_)))
            .collect::<Vec<_>>()
    };

    let whole = frames(parse_all(FLIGHT));
    for chunk in [1, 7, 64] {
        let chunked = frames(parse_streamed(FLIGHT, chunk));
        assert_eq!(chunked, whole, "chunk size {chunk}");
    }
}

#[test]
fn decodes_the_opening_sentences() {
    // A file snapshot of the first handful of sentences: the GNSS families
    // decoded, the FLARM ones kept as `Unknown`.
    let opening: Vec<Step> = parse_all(FLIGHT).into_iter().take(7).collect();
    insta::assert_debug_snapshot!(opening);
}

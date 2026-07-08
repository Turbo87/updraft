//! Snapshot test over a real recorded flight (`testdata/flight_1.nmea`,
//! ~4,200 sentences of GNSS, Garmin baro, and FLARM traffic). It asserts
//! that every line frames with a valid checksum and snapshots the tally of
//! `ParseResult` variants, so adding a sentence parser visibly shifts
//! counts out of `Unsupported`.

use std::collections::BTreeMap;

use updraft_nmea::ParseResult;

fn variant_name(result: &ParseResult) -> &'static str {
    match result {
        ParseResult::Gga(_) => "Gga",
        ParseResult::Rmc(_) => "Rmc",
        ParseResult::Gsa(_) => "Gsa",
        ParseResult::Pgrmz(_) => "Pgrmz",
        ParseResult::Pflau(_) => "Pflau",
        ParseResult::Pflaa(_) => "Pflaa",
        ParseResult::Unsupported => "Unsupported",
        _ => "Other",
    }
}

#[test]
fn flight_1_tally() {
    let corpus = include_str!("../testdata/flight_1.nmea");
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    let mut errors = 0usize;

    for line in corpus.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match updraft_nmea::parse(line) {
            Ok(result) => *tally.entry(variant_name(&result)).or_default() += 1,
            Err(_) => errors += 1,
        }
    }

    assert_eq!(
        errors, 0,
        "every corpus line should frame with a valid checksum"
    );
    insta::assert_debug_snapshot!(tally);
}

//! Property-based hardening: the parser must never panic on arbitrary
//! bytes and must always make progress until it needs more input.

use proptest::prelude::*;
use updraft_nmea::{Step, parse};

proptest! {
    #[test]
    fn never_panics_and_always_progresses(bytes: Vec<u8>) {
        let mut input = bytes.as_slice();
        loop {
            let before = input.len();
            if parse(&mut input) == Step::Incomplete {
                break;
            }

            // Every non-`Incomplete` step consumes at least one byte, so
            // the loop is guaranteed to terminate.
            prop_assert!(input.len() < before);
        }
    }

    #[test]
    fn arbitrary_bytes_before_a_sentence_still_yield_it(noise: Vec<u8>) {
        // Junk ahead of a valid sentence must be re-synchronized past, and
        // the sentence eventually recovered.

        let mut buffer = noise;
        buffer.extend_from_slice(b"\r\n$PGRMZ,4395,f,3*20\r\n");
        let mut input = buffer.as_slice();

        let mut framed = false;
        loop {
            match parse(&mut input) {
                Step::Incomplete => break,
                Step::Frame(_) => framed = true,
                Step::Rejected(_) => {}
            }
        }

        prop_assert!(framed);
    }
}

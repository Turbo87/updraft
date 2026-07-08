//! Property-based no-panic suite: parsing untrusted bytes (from a TCP or
//! Bluetooth link) must never panic, only return `Ok`/`Err`.

use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_never_panics_on_arbitrary_text(input in ".*") {
        let _ = updraft_nmea::parse(&input);
    }

    #[test]
    fn parse_never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let text = String::from_utf8_lossy(&bytes);
        let _ = updraft_nmea::parse(&text);
    }

    /// Well-framed lines with random payloads exercise the field parsers
    /// past the checksum gate rather than bouncing off it.
    #[test]
    fn parse_never_panics_on_framed_garbage(
        address in "[A-Z]{0,6}",
        payload in "[^*\r\n]{0,120}",
    ) {
        let body = format!("{address},{payload}");
        let checksum = updraft_nmea::checksum(body.as_bytes());
        let line = format!("${body}*{checksum:02X}");
        let _ = updraft_nmea::parse(&line);
    }
}

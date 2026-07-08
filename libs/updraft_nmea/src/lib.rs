//! NMEA 0183 sentence parsing for Updraft.
//!
//! This crate turns the line-oriented NMEA data emitted by GPS receivers,
//! varios, and FLARM units into the typed quantities from the
//! `updraft_units` and `updraft_geo` crates. It is deliberately built as
//! two pure, allocation-light pieces (see `docs/design/devices.md`):
//!
//! - [`Sentence::parse`] frames one line: it strips the `$`/`!` delimiter,
//!   verifies the `*HH` checksum, and exposes the address and fields.
//! - [`parse`] is the crate's **single entry point** for interpreting a
//!   sentence. It frames the line and routes it to the matching
//!   per-sentence parser, returning a [`ParseResult`] — one enum covering
//!   every sentence family the crate understands, plus an
//!   [`Unsupported`](ParseResult::Unsupported) variant for well-formed
//!   sentences it does not model.
//!
//! There is deliberately no parser registry or dispatcher here: a stream
//! that mixes GNSS, Garmin, and FLARM sentences is handled by calling
//! [`parse`] on each line and matching on the result. Routing sentences to
//! subsystems and tagging device capabilities is a device-layer concern
//! that lives with the `io-adapters` step.
//!
//! Every parser is a pure function over a borrowed string, so the whole
//! crate is trivially unit-testable and fuzzable; the test suite pairs
//! property-based no-panic checks with snapshot tests over a corpus of
//! recorded device captures.

mod datetime;
mod error;
mod fields;
mod framer;
mod scalars;
mod sentences;

pub use datetime::{Date, Time};
pub use error::ParseError;
pub use fields::Fields;
pub use framer::{Sentence, checksum};
pub use sentences::gga::{FixQuality, Gga};
pub use sentences::rmc::{Rmc, RmcStatus};

/// The result of interpreting one framed, checksum-valid NMEA sentence.
///
/// New sentence families are added as further variants over time, so the
/// enum is `#[non_exhaustive]`: downstream `match`es must include a
/// wildcard arm.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ParseResult {
    /// A `GGA` fix-data sentence.
    Gga(Gga),
    /// An `RMC` recommended-minimum sentence.
    Rmc(Rmc),
    /// A well-formed, checksum-valid sentence whose type the crate does
    /// not (yet) model.
    Unsupported,
}

/// Parse a single NMEA sentence line into a [`ParseResult`].
///
/// The line is framed and checksum-verified (see [`Sentence::parse`]) and
/// then routed to the parser for its sentence type. A well-formed sentence
/// the crate does not model yields [`ParseResult::Unsupported`]; a framing
/// failure or a malformed field yields the corresponding [`ParseError`].
pub fn parse(line: &str) -> Result<ParseResult, ParseError> {
    let sentence = Sentence::parse(line)?;
    route(&sentence)
}

/// Route a framed sentence to its per-sentence parser. As sentence
/// families are implemented, each gains an arm here; everything else falls
/// through to [`ParseResult::Unsupported`].
fn route(sentence: &Sentence<'_>) -> Result<ParseResult, ParseError> {
    let address = sentence.address();

    if let Some(formatter) = gnss_formatter(address) {
        return Ok(match formatter {
            "GGA" => ParseResult::Gga(sentences::gga::parse(sentence.fields())?),
            "RMC" => ParseResult::Rmc(sentences::rmc::parse(sentence.fields())?),
            _ => ParseResult::Unsupported,
        });
    }

    Ok(ParseResult::Unsupported)
}

/// For a standard GNSS address (`GPGGA`, `GNRMC`, …) return the
/// three-letter sentence formatter, accepting any GNSS talker ID plus the
/// nonstandard `BD` `BeiDou` talker (treated as an alias). Returns `None`
/// for proprietary or non-GNSS addresses.
fn gnss_formatter(address: &str) -> Option<&str> {
    let (talker, formatter) = address.split_at_checked(2)?;
    (formatter.len() == 3 && (talker.starts_with('G') || talker == "BD")).then_some(formatter)
}

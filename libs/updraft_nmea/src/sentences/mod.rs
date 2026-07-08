//! The per-sentence parsers. Each is a pure function from a [`Fields`]
//! cursor to a typed struct, reached through [`crate::parse`].
//!
//! [`Fields`]: crate::Fields

pub(crate) mod gga;

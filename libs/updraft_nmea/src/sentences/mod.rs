//! The per-sentence parsers. Each is a pure function from a [`Fields`]
//! cursor to a typed struct, reached through [`crate::parse`].
//!
//! [`Fields`]: crate::Fields

pub(crate) mod cai_w;
pub(crate) mod gga;
pub(crate) mod gsa;
pub(crate) mod lxwp0;
pub(crate) mod lxwp1;
pub(crate) mod lxwp2;
pub(crate) mod pcaib;
pub(crate) mod pcaid;
pub(crate) mod pflaa;
pub(crate) mod pflau;
pub(crate) mod pgrmz;
pub(crate) mod rmc;

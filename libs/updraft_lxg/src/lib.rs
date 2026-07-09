//! Reader and writer for LXNAV / LX Navigation `.lxg` glider-preset files.
//!
//! An `.lxg` file stores a single glider configuration for an LX80xx/90xx
//! flight computer: polar coefficients, reference and per-tank weights,
//! centre-of-gravity arms, ballast-dump tables, and per-flap speed ranges.
//!
//! The crate has two layers:
//!
//! * [`Glider`] — a high-level, named view (name, polar, masses, arms,
//!   speeds, flaps, ballast) that both reads
//!   ([`from_bytes`](Glider::from_bytes)) and writes
//!   ([`to_bytes`](Glider::to_bytes)). This is what you usually want. It is
//!   a lossy projection: reading drops unknown fields and exact on-wire
//!   types, and writing emits a canonical, normalized file.
//! * [`LxgFile`] / [`Section`] / [`Value`] — the faithful low-level
//!   representation, keyed by the device's raw config-register ids. Decode
//!   then encode reproduces a device-written file byte-for-byte, so reach
//!   for this layer when you must preserve every field of a specific file.
//!
//! ```no_run
//! use updraft_lxg::Glider;
//!
//! let mut glider = Glider::from_bytes(&std::fs::read("glider.lxg")?)?;
//! println!("{:?}: a={:?}", glider.name, glider.polar.a);
//! for flap in &glider.flaps {
//!     println!("  flap {} up to {:?} m/s", flap.label, flap.max_speed);
//! }
//!
//! glider.masses.empty = Some(352.0);
//! std::fs::write("glider.lxg", glider.to_bytes())?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # On-disk format
//!
//! The file is a bespoke tag-prefixed binary format. Its keys are the
//! device's config-register ids (the same namespace used on the
//! instrument's RS-485 bus).
//!
//! ```text
//! file    = 0xCC 0xFF                      version (a 0xCC-tagged byte, 255)
//!           section                        the root section
//! section = 0xDE <count:u16le>             entry count
//!           0xD5 0x2F <size:u16le>         size marker (skip hint)
//!           (key value){count}             key = 0xCD <u16le> register id
//!           0xC0                           terminator
//! ```
//!
//! Values are tag-prefixed and little-endian: `0xCA` f32, `0xCC` u8, `0xCD`
//! u16, `0xCE` u32, `0xD2` i32, `0xD9`/`0xDA` str, `0xC4`..`0xC6` bin,
//! `0xC7`/`0xC8` typed array (extension-type byte *before* the length).
//! The `size` marker is the byte distance from a value's introducing key
//! (or the version prefix, for the root) to the section terminator — a
//! redundant skip hint that this crate ignores on read and recomputes on
//! write. Numbers are stored in m/s (speeds), kg, mm (arms), litres and m²;
//! `-16384` (`0xFFFFC000`) is the "unset" sentinel.
//!
//! The tag byte values happen to coincide with [MessagePack]'s, but this is
//! not that format — it is little-endian, injects the `0xD5 0x2F` size
//! marker into every map header, swaps the ext type/length order, and
//! terminates maps with `0xC0`. An off-the-shelf decoder for that format
//! cannot read these files, which is why the reader here is hand-written.
//!
//! [MessagePack]: https://msgpack.org/

mod error;
mod glider;
mod reader;
mod value;
mod writer;

pub use error::Error;
pub use glider::{Arms, Ballast, Flap, Glider, Masses, Polar, Speeds};
pub use value::{Section, Value};

/// A decoded `.lxg` file: its format version and root section.
///
/// This is the low-level representation. For a friendlier read-only view
/// see [`glider`](Self::glider) / [`Glider`].
#[derive(Clone, Debug, PartialEq)]
pub struct LxgFile {
    /// The version number from the file's prefix. Every known file uses
    /// `255`.
    pub version: u8,
    /// The root section. In practice it holds a single `GLIDER_DEFS`
    /// (register 15900) entry.
    pub root: Section,
}

impl LxgFile {
    /// Decodes an `.lxg` file from its bytes.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the bytes are not a well-formed `.lxg` file.
    /// This never panics, whatever the input.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let (version, root) = reader::decode(bytes)?;
        Ok(LxgFile { version, root })
    }

    /// Encodes this file back to bytes.
    ///
    /// For a file produced by [`from_bytes`](Self::from_bytes) without
    /// modification, and originally written by an LX device, this
    /// reproduces the input byte-for-byte.
    pub fn to_bytes(&self) -> Vec<u8> {
        writer::encode(self.version, &self.root)
    }

    /// Extracts the high-level [`Glider`] view of this file.
    pub fn glider(&self) -> Glider {
        Glider::from_root(&self.root)
    }
}

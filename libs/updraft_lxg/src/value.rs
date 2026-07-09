use std::fmt;

/// A group of register fields — the `.lxg` equivalent of a map.
///
/// Entries are `(register id, value)` pairs kept in file order. The same
/// id can in principle appear more than once; [`Section::get`] returns the
/// first match. Keep the ordering stable when editing if you want the
/// output to match a device's own writer byte-for-byte.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Section {
    /// The register fields in this section, in file order.
    pub entries: Vec<(u16, Value)>,
}

impl Section {
    /// An empty section.
    pub fn new() -> Self {
        Self::default()
    }

    /// The value of the first entry with register id `id`, if any.
    pub fn get(&self, id: u16) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| *k == id).map(|(_, v)| v)
    }

    /// A mutable reference to the value of the first entry with register
    /// id `id`, if any.
    pub fn get_mut(&mut self, id: u16) -> Option<&mut Value> {
        self.entries
            .iter_mut()
            .find(|(k, _)| *k == id)
            .map(|(_, v)| v)
    }

    /// Sets the first entry with register id `id` to `value`, or appends a
    /// new entry if none exists. Returns the previous value, if any.
    pub fn set(&mut self, id: u16, value: Value) -> Option<Value> {
        if let Some(slot) = self.get_mut(id) {
            Some(std::mem::replace(slot, value))
        } else {
            self.entries.push((id, value));
            None
        }
    }
}

/// A single decoded field value.
///
/// The variants mirror the tag bytes the LX writer emits, keeping
/// each field's exact on-wire type so a decode/encode cycle reproduces the
/// original bytes. Integers keep their exact width because the same
/// logical number is stored as [`U8`](Value::U8), [`U16`](Value::U16) or
/// [`I32`](Value::I32) depending on the field; mixing them up would change
/// the bytes.
///
/// Numbers are little-endian on disk. Speeds are stored in m/s, masses in
/// kg, arms in mm, and `-16384` (`0xFFFFC000`) is the "unset" sentinel —
/// but those are field semantics this crate does not interpret; it only
/// preserves the raw values.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// Unsigned 8-bit integer (`0xCC`).
    U8(u8),
    /// Unsigned 16-bit integer (`0xCD`).
    U16(u16),
    /// Unsigned 32-bit integer (`0xCE`).
    U32(u32),
    /// Signed 32-bit integer (`0xD2`).
    I32(i32),
    /// 32-bit float (`0xCA`).
    F32(f32),
    /// UTF-8 string (`0xD9` / `0xDA`).
    Str(String),
    /// Opaque byte blob (`0xC4` / `0xC5` / `0xC6`); used for small flags.
    Bin(Vec<u8>),
    /// A typed array (`0xC7` / `0xC8`). `ext_type` is the LX extension tag
    /// and `data` is the raw payload. Float arrays (flap speeds, ballast
    /// tables, CG-envelope points) are little-endian `f32`s — see
    /// [`Value::as_f32_array`].
    Array {
        /// The LX extension type byte that precedes the payload.
        ext_type: u8,
        /// The raw payload bytes.
        data: Vec<u8>,
    },
    /// A nested section (`0xDE`).
    Section(Section),
}

impl Value {
    /// The float value, if this is an [`F32`](Value::F32).
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::F32(v) => Some(*v),
            _ => None,
        }
    }

    /// The value as an `i64`, if this is any integer variant.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::U8(v) => Some(i64::from(*v)),
            Value::U16(v) => Some(i64::from(*v)),
            Value::U32(v) => Some(i64::from(*v)),
            Value::I32(v) => Some(i64::from(*v)),
            _ => None,
        }
    }

    /// The string, if this is a [`Str`](Value::Str).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The nested section, if this is a [`Section`](Value::Section).
    pub fn as_section(&self) -> Option<&Section> {
        match self {
            Value::Section(s) => Some(s),
            _ => None,
        }
    }

    /// Interprets an [`Array`](Value::Array) payload as little-endian
    /// `f32`s. Returns `None` for non-array values or payloads whose length
    /// is not a multiple of four.
    pub fn as_f32_array(&self) -> Option<Vec<f32>> {
        let Value::Array { data, .. } = self else {
            return None;
        };
        if data.len() % 4 != 0 {
            return None;
        }
        Some(
            data.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        )
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::U8(v) => write!(f, "U8({v})"),
            Value::U16(v) => write!(f, "U16({v})"),
            Value::U32(v) => write!(f, "U32({v})"),
            Value::I32(v) => write!(f, "I32({v})"),
            Value::F32(v) => write!(f, "F32({v:?})"),
            Value::Str(s) => write!(f, "Str({s:?})"),
            // Render blobs and array payloads as hex so snapshots stay
            // compact and stable rather than long decimal byte lists.
            Value::Bin(b) => write!(f, "Bin(0x{})", hex(b)),
            Value::Array { ext_type, data } => {
                write!(
                    f,
                    "Array {{ ext_type: 0x{ext_type:02X}, data: 0x{} }}",
                    hex(data)
                )
            }
            Value::Section(s) => f.debug_tuple("Section").field(s).finish(),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    use fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

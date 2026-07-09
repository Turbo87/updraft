use crate::error::Error;
use crate::value::{Section, Value};

/// The version prefix every file starts with: a `0xCC`-tagged byte, 255.
pub(crate) const VERSION_PREFIX: [u8; 2] = [0xCC, 0xFF];

/// A bounds-checked forward cursor over the input. Every read either
/// advances within bounds or returns [`Error::UnexpectedEof`], so decoding
/// arbitrary bytes can fail but never panics.
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Cursor { bytes, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn u8(&mut self) -> Result<u8, Error> {
        let b = *self.bytes.get(self.pos).ok_or(Error::UnexpectedEof)?;
        self.pos += 1;
        Ok(b)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::UnexpectedEof)?;
        let slice = self.bytes.get(self.pos..end).ok_or(Error::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }

    fn u16_le(&mut self) -> Result<u16, Error> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32_le(&mut self) -> Result<u32, Error> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn string(&mut self, len: usize) -> Result<Value, Error> {
        let bytes = self.take(len)?;
        let s = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
        Ok(Value::Str(s.to_owned()))
    }

    /// Reads one value at the current position.
    fn value(&mut self) -> Result<Value, Error> {
        let tag = self.u8()?;
        match tag {
            0xCC => Ok(Value::U8(self.u8()?)),
            0xCD => Ok(Value::U16(self.u16_le()?)),
            0xCE => Ok(Value::U32(self.u32_le()?)),
            0xD2 => {
                let b = self.take(4)?;
                Ok(Value::I32(i32::from_le_bytes([b[0], b[1], b[2], b[3]])))
            }
            0xCA => {
                let b = self.take(4)?;
                Ok(Value::F32(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
            }
            // str8 / str16, with a little-endian length like everything
            // else in this format.
            0xD9 => {
                let len = usize::from(self.u8()?);
                self.string(len)
            }
            0xDA => {
                let len = usize::from(self.u16_le()?);
                self.string(len)
            }
            // bin8 / bin16 / bin32.
            0xC4 => {
                let len = usize::from(self.u8()?);
                Ok(Value::Bin(self.take(len)?.to_vec()))
            }
            0xC5 => {
                let len = usize::from(self.u16_le()?);
                Ok(Value::Bin(self.take(len)?.to_vec()))
            }
            0xC6 => {
                let len = self.u32_le()? as usize;
                Ok(Value::Bin(self.take(len)?.to_vec()))
            }
            // ext8 / ext16 with the LX quirk: the extension type byte comes
            // *before* the length, and the length is little-endian.
            0xC7 => {
                let ext_type = self.u8()?;
                let len = usize::from(self.u8()?);
                Ok(Value::Array {
                    ext_type,
                    data: self.take(len)?.to_vec(),
                })
            }
            0xC8 => {
                let ext_type = self.u8()?;
                let len = usize::from(self.u16_le()?);
                Ok(Value::Array {
                    ext_type,
                    data: self.take(len)?.to_vec(),
                })
            }
            0xDE => self.section().map(Value::Section),
            other => Err(Error::UnsupportedTag(other)),
        }
    }

    /// Reads a section body, assuming the `0xDE` tag has been consumed.
    ///
    /// Layout: `<count:u16 LE> 0xD5 0x2F <size:u16 LE> {(key, value) * count} 0xC0`.
    /// The `size` marker is a redundant skip hint (bytes from the
    /// introducing key to the terminator); it is recomputed on write and
    /// ignored here.
    fn section(&mut self) -> Result<Section, Error> {
        let count = self.u16_le()?;
        if self.u8()? != 0xD5 || self.u8()? != 0x2F {
            return Err(Error::MalformedSectionHeader);
        }
        let _size = self.u16_le()?;
        let mut entries = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let key = match self.value()? {
                Value::U16(id) => id,
                _ => return Err(Error::NonIntegerKey),
            };
            let value = self.value()?;
            entries.push((key, value));
        }
        if self.u8()? != 0xC0 {
            return Err(Error::MissingSectionTerminator);
        }
        Ok(Section { entries })
    }
}

/// Decodes a whole `.lxg` file into its version byte and root section.
pub(crate) fn decode(bytes: &[u8]) -> Result<(u8, Section), Error> {
    if bytes.first() != Some(&VERSION_PREFIX[0]) {
        return Err(Error::MissingVersion);
    }
    let mut cursor = Cursor::new(bytes);
    cursor.u8()?; // 0xCC
    let version = cursor.u8()?; // version number (0xFF in known files)

    if cursor.u8()? != 0xDE {
        return Err(Error::MalformedSectionHeader);
    }
    let root = cursor.section()?;

    if cursor.remaining() != 0 {
        return Err(Error::TrailingData);
    }
    Ok((version, root))
}

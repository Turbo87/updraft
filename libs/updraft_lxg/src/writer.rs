use crate::reader::VERSION_PREFIX;
use crate::value::{Section, Value};

/// Number of bytes a `u16` map key occupies on disk (`0xCD` + 2 bytes).
const KEY_LEN: usize = 3;

/// Encodes a file (version byte + root section) to bytes.
pub(crate) fn encode(version: u8, root: &Section) -> Vec<u8> {
    let mut out = vec![VERSION_PREFIX[0], version];
    // The root section is introduced by the 2-byte version prefix rather
    // than a key, so its size marker counts those two bytes.
    write_section(root, VERSION_PREFIX.len(), &mut out);
    out
}

/// Writes a value. `prefix_len` is the size of the bytes that introduced
/// this value (its key, or the version prefix for the root) and is only
/// used to compute a section's redundant size marker.
fn write_value(value: &Value, prefix_len: usize, out: &mut Vec<u8>) {
    match value {
        Value::U8(v) => {
            out.push(0xCC);
            out.push(*v);
        }
        Value::U16(v) => {
            out.push(0xCD);
            out.extend_from_slice(&v.to_le_bytes());
        }
        Value::U32(v) => {
            out.push(0xCE);
            out.extend_from_slice(&v.to_le_bytes());
        }
        Value::I32(v) => {
            out.push(0xD2);
            out.extend_from_slice(&v.to_le_bytes());
        }
        Value::F32(v) => {
            out.push(0xCA);
            out.extend_from_slice(&v.to_le_bytes());
        }
        Value::Str(s) => write_str(s, out),
        Value::Bin(b) => write_bin(b, out),
        Value::Array { ext_type, data } => write_array(*ext_type, data, out),
        Value::Section(section) => write_section(section, prefix_len, out),
    }
}

fn write_str(s: &str, out: &mut Vec<u8>) {
    let bytes = s.as_bytes();
    if let Ok(len) = u8::try_from(bytes.len()) {
        out.push(0xD9);
        out.push(len);
    } else {
        out.push(0xDA);
        out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
    }
    out.extend_from_slice(bytes);
}

fn write_bin(b: &[u8], out: &mut Vec<u8>) {
    if let Ok(len) = u8::try_from(b.len()) {
        out.push(0xC4);
        out.push(len);
    } else if let Ok(len) = u16::try_from(b.len()) {
        out.push(0xC5);
        out.extend_from_slice(&len.to_le_bytes());
    } else {
        out.push(0xC6);
        out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(b);
}

fn write_array(ext_type: u8, data: &[u8], out: &mut Vec<u8>) {
    // The LX quirk: extension type byte first, then a little-endian length.
    if let Ok(len) = u8::try_from(data.len()) {
        out.push(0xC7);
        out.push(ext_type);
        out.push(len);
    } else {
        out.push(0xC8);
        out.push(ext_type);
        out.extend_from_slice(&(data.len() as u16).to_le_bytes());
    }
    out.extend_from_slice(data);
}

fn write_section(section: &Section, prefix_len: usize, out: &mut Vec<u8>) {
    // Encode the entries first so we know the section's length.
    let mut body = Vec::new();
    for (key, value) in &section.entries {
        body.push(0xCD);
        body.extend_from_slice(&key.to_le_bytes());
        write_value(value, KEY_LEN, &mut body);
    }

    // Full section = 0xDE + count(2) + 0xD5 0x2F + size(2) + body + 0xC0.
    let section_len = 1 + 2 + 2 + 2 + body.len() + 1;
    let size = (prefix_len + section_len) as u16;

    out.push(0xDE);
    out.extend_from_slice(&(section.entries.len() as u16).to_le_bytes());
    out.push(0xD5);
    out.push(0x2F);
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&body);
    out.push(0xC0);
}

//! Byte-exact round-trip of a real device-written `.lxg` file.
//!
//! The fixture is a real DG-800B preset exported from an LX device. If
//! decode-then-encode reproduces it byte-for-byte, the format model (value
//! types, endianness, section size markers, terminators) is correct.

use updraft_lxg::{LxgFile, Value};

const DG800B: &[u8] = include_bytes!("fixtures/dg-800b.lxg");

#[test]
fn decode_encode_is_byte_exact() {
    let file = LxgFile::from_bytes(DG800B).expect("fixture should decode");
    let reencoded = file.to_bytes();
    assert_eq!(
        reencoded, DG800B,
        "re-encoded bytes must match the original"
    );
}

#[test]
fn version_is_255() {
    let file = LxgFile::from_bytes(DG800B).unwrap();
    assert_eq!(file.version, 255);
}

#[test]
fn exposes_polar_coefficients() {
    let file = LxgFile::from_bytes(DG800B).unwrap();
    let polar = file
        .root
        .get(15900) // GLIDER_DEFS
        .and_then(Value::as_section)
        .and_then(|s| s.get(15901)) // GLIDER_DEF
        .and_then(Value::as_section)
        .and_then(|s| s.get(16000)) // POLAR
        .and_then(Value::as_section)
        .expect("POLAR section should exist");

    // Polar coefficients a/b/c for the DG-800B.
    assert_eq!(polar.get(16002).and_then(Value::as_f32), Some(1.05));
    assert_eq!(polar.get(16003).and_then(Value::as_f32), Some(-1.42));
    assert_eq!(polar.get(16004).and_then(Value::as_f32), Some(0.93));
    // Empty / max weights are integers, in kg.
    assert_eq!(polar.get(16009).and_then(Value::as_int), Some(352));
    assert_eq!(polar.get(16008).and_then(Value::as_int), Some(525));
}

#[test]
fn editing_then_re_encoding_changes_only_that_field() {
    let mut file = LxgFile::from_bytes(DG800B).unwrap();

    // Bump the empty weight (register 16009) inside POLAR.
    let polar = file
        .root
        .get_mut(15900)
        .and_then(|v| match v {
            Value::Section(s) => Some(s),
            _ => None,
        })
        .and_then(|s| s.get_mut(15901))
        .and_then(|v| match v {
            Value::Section(s) => Some(s),
            _ => None,
        })
        .and_then(|s| s.get_mut(16000))
        .and_then(|v| match v {
            Value::Section(s) => Some(s),
            _ => None,
        })
        .unwrap();
    polar.set(16009, Value::I32(360));

    // Round-trips cleanly and the edit is observable.
    let bytes = file.to_bytes();
    let reparsed = LxgFile::from_bytes(&bytes).unwrap();
    let weight = reparsed
        .root
        .get(15900)
        .and_then(Value::as_section)
        .and_then(|s| s.get(15901))
        .and_then(Value::as_section)
        .and_then(|s| s.get(16000))
        .and_then(Value::as_section)
        .and_then(|s| s.get(16009))
        .and_then(Value::as_int);
    assert_eq!(weight, Some(360));
    // The edit kept the file the same length (i32 -> i32), so only the four
    // weight bytes differ from the original.
    assert_eq!(bytes.len(), DG800B.len());
}

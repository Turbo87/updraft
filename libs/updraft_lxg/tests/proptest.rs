//! Property tests.
//!
//! The primary property is panic-freedom: [`LxgFile::from_bytes`] must
//! return `Ok`/`Err` for *any* input, never panic (no slice-index or
//! integer-overflow crashes). The secondary property is that encoding is a
//! stable round-trip for any well-formed value tree.

use proptest::prelude::*;
use updraft_lxg::{LxgFile, Section, Value};

/// A strategy for arbitrary values, bounded so a generated tree stays well
/// under the `u16` section-size limit and only exercises the narrow
/// encodings real files use.
fn value_strategy() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        any::<u8>().prop_map(Value::U8),
        any::<u16>().prop_map(Value::U16),
        any::<u32>().prop_map(Value::U32),
        any::<i32>().prop_map(Value::I32),
        any::<f32>().prop_map(Value::F32),
        "[ -~]{0,32}".prop_map(Value::Str),
        proptest::collection::vec(any::<u8>(), 0..16).prop_map(Value::Bin),
        (any::<u8>(), proptest::collection::vec(any::<u8>(), 0..16))
            .prop_map(|(ext_type, data)| Value::Array { ext_type, data }),
    ];

    leaf.prop_recursive(4, 32, 6, |inner| {
        proptest::collection::vec((any::<u16>(), inner), 0..6)
            .prop_map(|entries| Value::Section(Section { entries }))
    })
}

fn section_strategy() -> impl Strategy<Value = Section> {
    proptest::collection::vec((any::<u16>(), value_strategy()), 0..8)
        .prop_map(|entries| Section { entries })
}

proptest! {
    /// Arbitrary bytes never make the decoder panic.
    #[test]
    fn from_bytes_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = LxgFile::from_bytes(&bytes);
    }

    /// Arbitrary bytes behind a valid version prefix never panic either —
    /// this drives the section parser far past the shallow rejections.
    #[test]
    fn from_bytes_never_panics_after_version(rest in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let mut bytes = vec![0xCC, 0xFF];
        bytes.extend(rest);
        let _ = LxgFile::from_bytes(&bytes);
    }

    /// Encoding any well-formed file and decoding it back yields an
    /// identical byte stream (encode is a stable fixed point). Comparing
    /// bytes rather than values sidesteps `NaN != NaN`.
    #[test]
    fn encode_decode_is_stable(version in any::<u8>(), root in section_strategy()) {
        let file = LxgFile { version, root };
        let bytes = file.to_bytes();
        let decoded = LxgFile::from_bytes(&bytes).expect("self-encoded file must decode");
        prop_assert_eq!(decoded.to_bytes(), bytes);
    }
}

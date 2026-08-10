/// Declares an OpenAIP numeric code enum.
///
/// OpenAIP encodes classifications as integers. It adds codes to the model
/// without a version change. Each generated enum therefore keeps an unsupported
/// code in `Unsupported` instead of failing the complete dataset.
macro_rules! codes {
    (
        $(#[$enum_meta:meta])*
        pub enum $name:ident {
            $($(#[$variant_meta:meta])* $code:literal => $variant:ident,)+
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize)]
        #[serde(from = "u16")]
        pub enum $name {
            $($(#[$variant_meta])* $variant,)+
            /// A code that this crate does not support.
            Unsupported(u16),
        }

        impl From<u16> for $name {
            fn from(code: u16) -> Self {
                match code {
                    $($code => Self::$variant,)+
                    unsupported => Self::Unsupported(unsupported),
                }
            }
        }
    };
}

pub(crate) use codes;

#[cfg(test)]
mod tests {
    use claims::assert_ok_eq;

    codes! {
        /// A classification with one documented code.
        pub enum Example {
            0 => Documented,
        }
    }

    #[test]
    fn deserializes_a_documented_code() {
        assert_ok_eq!(serde_json::from_str::<Example>("0"), Example::Documented);
    }

    #[test]
    fn keeps_an_undocumented_code() {
        assert_ok_eq!(
            serde_json::from_str::<Example>("7"),
            Example::Unsupported(7)
        );
    }
}

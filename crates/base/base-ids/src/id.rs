//! The grammar of an id, and the macro that declares one: opaque, stable, ASCII slugs, parsed
//! once where they enter. The ids themselves are declared, with [`slug_id!`], by the crate that
//! owns the concept they name.
//!
//! One grammar for every id (`[a-z0-9]` then `[a-z0-9._-]`, at most 64 bytes) so an id is
//! always safe in a file name, a Secret Service attribute and, after [`object_segment`],
//! a D-Bus object path. A UUID in its hyphenated lowercase form is a valid id.

/// The longest id, in bytes.
const MAX_LEN: usize = 64;

/// Whether `text` is a well-formed id (the one grammar every id of ours shares; crates that
/// mint ids of their own, such as `prov`, check with this).
#[must_use]
pub fn is_id(text: &str) -> bool {
    let mut bytes = text.bytes();
    let first_ok = bytes
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    first_ok
        && text.len() <= MAX_LEN
        && bytes.all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

/// The text-newtype items every validated string shares.
///
/// They are for a tuple struct `Name(String)` (or `Name(Box<str>)`) whose `parse(&str) ->
/// Result<Self, $error>` holds the rule: `as_str`, `TryFrom<String>` (so serde's `try_from =
/// "String"` runs `parse`), `From<Name> for String` and `Display`. The one place those four are
/// written.
#[macro_export]
macro_rules! string_newtype {
    ($name:ident, $error:ty) => {
        impl $name {
            /// The text as written.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = $error;
            fn try_from(text: String) -> Result<Self, $error> {
                Self::parse(&text)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0.into()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// Declares an id type in the one slug grammar ([`is_id`]).
///
/// It is a `String` newtype whose `parse` is the only way in, whose serde form is its text, and
/// whose `Display` writes the text. The field is private to the module that declares it, and the
/// caller depends on `serde`.
#[macro_export]
macro_rules! slug_id {
    ($(#[$doc:meta])* $name:ident, $what:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// The id written as `text`, or why it is not one.
            pub fn parse(text: &str) -> Result<Self, $crate::IdError> {
                if $crate::is_id(text) {
                    Ok(Self(text.to_owned()))
                } else {
                    Err($crate::IdError::malformed($what, text))
                }
            }
        }

        $crate::string_newtype!($name, $crate::IdError);
    };
}

/// Whether `text` is two or more dot-separated elements, each `[a-z][a-z0-9_]*`
/// (`mail.thread`, `mail.thread.archive`).
#[must_use]
pub fn is_dotted_name(text: &str) -> bool {
    let element = |e: &str| {
        let mut bytes = e.bytes();
        bytes.next().is_some_and(|b| b.is_ascii_lowercase())
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };
    let mut count = 0;
    let all = text.split('.').all(|e| {
        count += 1;
        element(e)
    });
    all && count >= 2
}

/// Whether `text` is 1 to `max` bytes with no control characters: text that an id does not
/// interpret.
#[must_use]
pub fn is_opaque(text: &str, max: usize) -> bool {
    !text.is_empty() && text.len() <= max && !text.chars().any(char::is_control)
}

/// Declares an id type with a grammar of its own: `ok` is a `fn(&str) -> bool`.
///
/// Like [`slug_id!`] otherwise: a `String` newtype whose `parse` is the only way in, whose serde
/// form is its text, and whose `Display` writes the text. The caller depends on `serde`.
#[macro_export]
macro_rules! text_id {
    ($(#[$doc:meta])* $name:ident, $what:literal, $ok:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// The id written as `text`, or why it is not one.
            pub fn parse(text: &str) -> Result<Self, $crate::IdError> {
                let ok: fn(&str) -> bool = $ok;
                if ok(text) {
                    Ok(Self(text.to_owned()))
                } else {
                    Err($crate::IdError::malformed($what, text))
                }
            }
        }

        $crate::string_newtype!($name, $crate::IdError);
    };
}

/// The id as one D-Bus object path segment (`[A-Za-z0-9_]`): `.` and `-` become `_`.
///
/// Two ids differing only in those characters share a segment; accountd refuses the second
/// (it never mints such ids itself).
#[must_use]
pub fn object_segment(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    slug_id!(
        /// An id of the tests.
        TestId,
        "test id"
    );

    text_id!(
        /// A dotted id of the tests.
        DottedId,
        "dotted id",
        is_dotted_name
    );

    #[test]
    fn dotted_names_need_two_lowercase_elements() {
        for (text, ok) in [
            ("mail.thread", true),
            ("mail.thread.archive", true),
            ("mail.thread_x9", true),
            ("mail", false),
            ("Mail.thread", false),
            ("mail..thread", false),
            ("mail.1thread", false),
            ("", false),
        ] {
            assert_eq!(is_dotted_name(text), ok, "{text:?}");
            assert_eq!(DottedId::parse(text).is_ok(), ok, "{text:?}");
        }
    }

    #[test]
    fn opaque_text_is_bounded_and_has_no_control_characters() {
        assert!(is_opaque("a/b c", 5));
        assert!(!is_opaque("", 5));
        assert!(!is_opaque("abcdef", 5));
        assert!(!is_opaque("a\nb", 5));
    }

    const CASES: &[(&str, &str, bool)] = &[
        ("plain", "nextcloud", true),
        ("uuid", "67e55044-10b1-426f-9247-bb680e5fe0c8", true),
        ("dots", "llama3.2", true),
        ("empty", "", false),
        ("upper", "Nextcloud", false),
        ("leading dash", "-x", false),
        ("slash", "a/b", false),
        ("space", "a b", false),
        (
            "too long",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            false,
        ),
    ];

    #[test]
    fn ids_parse_only_the_slug_grammar() {
        for (name, text, ok) in CASES {
            assert_eq!(TestId::parse(text).is_ok(), *ok, "{name}");
        }
    }

    #[test]
    fn a_malformed_id_does_not_deserialize() {
        let parsed: Result<TestId, _> = serde_json::from_str("\"Not An Id\"");
        parsed.unwrap_err();
    }

    #[test]
    fn object_segment_keeps_only_path_characters() {
        assert_eq!(object_segment("67e55044-10b1.x"), "67e55044_10b1_x");
    }
}

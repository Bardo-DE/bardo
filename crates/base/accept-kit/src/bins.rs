//! The binaries `cargo xtask accept` builds before a suite runs.

use std::path::PathBuf;

/// The environment variable that holds the path of the prebuilt binary `name`:
/// `bardo-accountd` is `BARDO_ACCEPT_BIN_BARDO_ACCOUNTD`.
#[must_use]
pub fn bin_var(name: &str) -> String {
    let upper: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("BARDO_ACCEPT_BIN_{upper}")
}

/// The path of the prebuilt binary `name`, as `cargo xtask accept` exported it.
///
/// # Panics
/// When the variable is unset (the test was not started through `cargo xtask accept <suite>`) or
/// the file is not there.
#[must_use]
pub fn bin(name: &str) -> PathBuf {
    let var = bin_var(name);
    let path = std::env::var_os(&var).map_or_else(
        || {
            panic!(
                "{var} is not set: run through `cargo xtask accept <suite>`, which builds {name}"
            )
        },
        PathBuf::from,
    );
    assert!(
        path.is_file(),
        "{var} names {}, which is not a file",
        path.display()
    );
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variable_is_the_upper_snake_name() {
        assert_eq!(bin_var("bardo-accountd"), "BARDO_ACCEPT_BIN_BARDO_ACCOUNTD");
        assert_eq!(bin_var("fake-logind"), "BARDO_ACCEPT_BIN_FAKE_LOGIND");
        assert_eq!(
            bin_var("accept_bardo.do"),
            "BARDO_ACCEPT_BIN_ACCEPT_BARDO_DO"
        );
    }

    #[test]
    #[should_panic(expected = "run through `cargo xtask accept <suite>`")]
    fn an_unset_variable_says_how_to_run() {
        let _ = bin("no-such-binary-for-this-test");
    }
}

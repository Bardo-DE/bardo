//! What of syncd's places the daemon decides itself: the build (the one knob a release must not
//! have) and how often the supervisors look again. The paths themselves are `sync_xdg`.

use std::time::Duration;

/// Which build this is, for the one knob a release must not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    /// Built with the `test-proc-root` feature.
    Test,
    /// Every other build: the shipped one.
    Release,
}

/// The build this binary is.
pub const BUILD: Build = if cfg!(feature = "test-proc-root") {
    Build::Test
} else {
    Build::Release
};

/// Whether this binary honours `BARDO_SYNCD_PROC_ROOT`: only with the `test-proc-root` feature. A
/// release build ignores the variable.
pub const HONOURS_PROC_ROOT: bool = cfg!(feature = "test-proc-root");

/// How often the supervisors read grants again.
///
/// Read from `BARDO_SYNCD_RESCAN_S` (whole seconds, at least 1), for a test build only, so a scenario need not wait the default ten minutes for a grant it
/// just made. A release build ignores the variable; so does a value that is not a number of
/// seconds. The default is `default`, and no build has a shorter one.
pub fn rescan(build: Build, value: Option<String>, default: Duration) -> Duration {
    match build {
        Build::Test => value
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|seconds| *seconds >= 1)
            .map_or(default, Duration::from_secs),
        Build::Release => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rescan_variable_is_read_by_a_test_build_only_and_never_shortens_the_default() {
        let default = Duration::from_secs(600);
        let cases = [
            (Build::Test, Some("2"), Duration::from_secs(2)),
            (Build::Test, Some(" 5 "), Duration::from_secs(5)),
            (Build::Test, Some("0"), default),
            (Build::Test, Some("soon"), default),
            (Build::Test, Some("-3"), default),
            (Build::Test, None, default),
            (Build::Release, Some("2"), default),
            (Build::Release, None, default),
        ];
        for (build, value, want) in cases {
            assert_eq!(
                rescan(build, value.map(str::to_owned), default),
                want,
                "{build:?} {value:?}"
            );
        }
    }

    #[test]
    fn a_shipped_build_is_a_release_build() {
        let manifest = include_str!("../Cargo.toml");
        let default = manifest
            .lines()
            .skip_while(|l| !l.starts_with("[features]"))
            .any(|l| l.trim_start().starts_with("default"));
        assert!(
            !default,
            "no feature is default, so test-proc-root never is"
        );
    }
}

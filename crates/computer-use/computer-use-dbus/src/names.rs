//! Bus names and object paths.

use provenance::RunId;

/// cuad's bus name.
pub const CUA_BUS: &str = "org.bardo.ComputerUse1";
/// cuad's root object (`Manager`).
pub const CUA_PATH: &str = "/org/bardo/ComputerUse1";
/// The bus name cuad also owns, as the provider of the `computeruse.run.start` action
/// (`org.bardo.IntentProvider1` at the actions service's `PROVIDER_PATH`).
pub const PROVIDER_BUS: &str = "org.bardo.ComputerUse";

/// The object path of one run (`/org/bardo/ComputerUse1/run/<id>`).
///
/// A run id is a slug with `.`, `_` and `-`, of which an object path allows only `_`, so `.` and
/// `-` become `_` (`r-4.a` is `r_4_a`). The mapping loses the original, so a client reads the `Id`
/// property of the run object (the `RunId` slug) rather than parsing the path; `RunAdded` and
/// `RunRemoved` carry the path.
#[must_use]
pub fn run_path(run: &RunId) -> String {
    let segment: String = run
        .as_str()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{CUA_PATH}/run/{segment}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_paths_are_valid_object_paths() {
        for (id, want) in [
            ("r-1", "/org/bardo/ComputerUse1/run/r_1"),
            ("r-4.a", "/org/bardo/ComputerUse1/run/r_4_a"),
        ] {
            let path = run_path(&RunId::parse(id).unwrap());
            assert_eq!(path, want);
            zbus::zvariant::ObjectPath::try_from(path.as_str()).unwrap();
        }
    }
}

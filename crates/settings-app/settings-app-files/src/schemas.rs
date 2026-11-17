//! Finding the schemas programs ship (design/22 section 9.2 and 9.3 item 1): the folders are read
//! here, and the model checks what was found (`settings_app_model::admit_scans`).

use settings_app_model::{Discovery, OriginRule, SchemaOrigin, admit_scans};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Every schema under `dirs`, judged by who owns the folders ([`OriginRule::Ownership`]).
#[must_use]
pub fn discover(dirs: &[PathBuf]) -> Discovery {
    discover_under(dirs, &OriginRule::Ownership)
}

/// Every schema under `dirs` (`settings_store_fs::data_dirs`: the system's directories, then
/// `$XDG_DATA_HOME`'s, in the order of trust), the folders judged by `rule`.
///
/// `settings_store_fs` admits a schema only for its own program and location (the first one found
/// for a program wins, so the user's directory never replaces an installed schema) and reports the
/// rest on stderr. It does not say which folder a schema came from, so each folder is scanned on
/// its own and judged on its own; the model takes the first schema of each program.
#[must_use]
pub fn discover_under(dirs: &[PathBuf], rule: &OriginRule) -> Discovery {
    admit_scans(dirs.iter().map(|dir| {
        (
            judge(rule, dir),
            settings_store_fs::discover(std::slice::from_ref(dir)),
        )
    }))
}

/// The origin of `dir` under `rule`.
#[must_use]
pub fn judge(rule: &OriginRule, dir: &Path) -> SchemaOrigin {
    rule.declared_origin(dir)
        .unwrap_or_else(|| origin_of_dir(dir))
}

/// The origin of `dir`: `System` only when the folder and every parent above it (after links
/// are resolved) are root's and closed to everyone else, since a writable parent could swap
/// the folder. A folder that cannot be read is the person's.
#[must_use]
pub fn origin_of_dir(dir: &Path) -> SchemaOrigin {
    let Ok(real) = dir.canonicalize() else {
        return SchemaOrigin::PersonWritable;
    };
    let closed = real.ancestors().all(|folder| {
        folder.metadata().is_ok_and(|meta| {
            SchemaOrigin::of_owner(meta.uid(), meta.mode()) == SchemaOrigin::System
        })
    });
    if closed {
        SchemaOrigin::System
    } else {
        SchemaOrigin::PersonWritable
    }
}

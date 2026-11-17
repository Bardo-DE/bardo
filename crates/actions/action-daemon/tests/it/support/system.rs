//! The administrator's files of a test: its own system, in the test's scratch directory, so that
//! no test reads this computer's `/etc/bardo`.

use action_daemon::SystemFiles;
use std::path::Path;

/// A system in `<dir>/system`: the callers table the package ships, no floor and no policy
/// folder.
pub fn scratch(dir: &Path) -> SystemFiles {
    let system = dir.join("system");
    std::fs::create_dir_all(&system).expect("system dir");
    let files = SystemFiles::in_dir(&system);
    std::fs::write(&files.callers, action_daemon::callers::SHIPPED).expect("system callers");
    files
}

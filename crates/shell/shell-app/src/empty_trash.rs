//! The daemon's half of the Empty Trash question.
//!
//! The `EmptyTrash` action opens it on the link (`actions::panel::empty_trash`), the surface shows
//! it, and a yes empties the home Trash on a thread of its own (`shell_desktop_fs::trash`). Cancel
//! sends nothing, so nothing is erased.

use shell_model::dirs::Dirs;
use shell_ui_kit::links::EmptyTrashLink;
use tokio::runtime::Handle;

/// Follow the yes on `link` on `rt` and empty the Trash for each one.
pub fn spawn_follower(rt: &Handle, link: &EmptyTrashLink) {
    let mut yes = link.subscribe_answered();
    rt.spawn(async move {
        while yes.changed().await.is_ok() {
            if yes.borrow_and_update().is_some() {
                shell_desktop_fs::trash::empty_in_background(Dirs::from_process());
            }
        }
    });
}

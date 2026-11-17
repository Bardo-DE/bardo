//! The application index of the desktop entries and their icons.
//!
//! The application index: desktop entries from every XDG applications directory, their icons
//! resolved once, rescanned on change, through `ui_helpers::DesktopFile` and freedesktop-icons; the
//! theme from GTK's settings, else hicolor;
//! `notify` on the applications directories, 500 ms.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::apps::AppsService;
use shell_model::service::ServiceIo;

pub use backend::{parse_entry, scan};

impl Run for AppsService {
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let config = env.dirs.config.clone();
        let dirs = env.dirs.applications.clone();
        let roots = env.dirs.icon_file_roots();
        async move {
            backend::run(io.state, io.commands, config, dirs, roots).await;
            Ok(())
        }
    }
}

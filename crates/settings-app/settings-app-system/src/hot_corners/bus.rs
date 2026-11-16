//! This computer's manifests of installed apps, and the desktop's own file chooser.

use super::CornerDesk;
use crate::chooser::{OnPortal, Want, choose};
use settings_app_files::read_manifests;
use settings_app_model::{HotActionChoice, HotOpenKind, Work, actions_of_all};
use std::path::PathBuf;
use std::sync::Arc;

/// The manifest folders on disk and the desktop's chooser.
#[derive(Debug)]
pub struct OnCorners {
    chooser: Arc<OnPortal>,
    intent_dirs: Vec<PathBuf>,
}

impl OnCorners {
    /// The manifests under `intent_dirs` (the folders of trust, first first), files chosen
    /// through `chooser`.
    #[must_use]
    pub fn new(chooser: Arc<OnPortal>, intent_dirs: Vec<PathBuf>) -> OnCorners {
        OnCorners {
            chooser,
            intent_dirs,
        }
    }
}

impl CornerDesk for OnCorners {
    fn actions(&self) -> Work<'_, Vec<HotActionChoice>> {
        let dirs = self.intent_dirs.clone();
        Box::pin(async move {
            // Reading the files takes a while: on a thread of its own, never the window's.
            tokio::task::spawn_blocking(move || {
                let texts = read_manifests(&dirs);
                actions_of_all(texts.iter().map(String::as_str))
            })
            .await
            .unwrap_or_default()
        })
    }

    fn choose(&self, kind: HotOpenKind) -> Work<'_, Option<String>> {
        Box::pin(async move {
            let want = match kind {
                HotOpenKind::File => Want::File,
                HotOpenKind::Folder => Want::Folder,
                HotOpenKind::App | HotOpenKind::Web => return None,
            };
            choose(self.chooser.as_ref(), want).await
        })
    }
}

//! What the desktop says, read from the settings portal by `ui-settings`
//! (`org.freedesktop.appearance`: color-scheme, reduced motion, contrast).

use crate::appearance::theme::Scheme;
use serde::{Deserialize, Serialize};

/// The desktop's contrast preference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum Contrast {
    /// Ordinary contrast.
    #[default]
    Normal,
    /// The desktop asks for higher contrast.
    High,
}

impl Contrast {
    /// The `data-contrast` value a root carries: only increased contrast writes one, so a
    /// root at ordinary contrast is marked as it always was.
    pub const fn attribute(self) -> Option<&'static str> {
        match self {
            Contrast::Normal => None,
            Contrast::High => Some("high"),
        }
    }
}

/// Everything the desktop contributes to resolving an [`crate::Appearance`].
///
/// The default is what a desktop with no portal answers: light, no motion preference, normal
/// contrast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash, Default)]
#[serde(default)]
pub struct SystemPrefs {
    /// The desktop's colour scheme.
    pub scheme: Scheme,
    /// Whether the desktop asks for reduced motion.
    pub reduced_motion: bool,
    /// The desktop's contrast preference.
    pub contrast: Contrast,
}

impl SystemPrefs {
    /// These preferences with the colour scheme `scheme`.
    #[must_use]
    pub const fn with_scheme(mut self, scheme: Scheme) -> Self {
        self.scheme = scheme;
        self
    }

    /// These preferences with the desktop's reduced-motion preference `reduced`.
    #[must_use]
    pub const fn with_reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
        self
    }

    /// These preferences with the contrast preference `contrast`.
    #[must_use]
    pub const fn with_contrast(mut self, contrast: Contrast) -> Self {
        self.contrast = contrast;
        self
    }
}

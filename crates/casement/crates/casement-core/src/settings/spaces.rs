//! Space settings (`spaces.*`, 03 §1.17).

/// `spaces.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceSettings {
    /// `spaces.per_display`: each display has its own Spaces.
    pub per_display: bool,
    /// `spaces.auto_rearrange`: Spaces reorder by recent use (open D4).
    pub auto_rearrange: bool,
    /// `spaces.switch_on_activate`: activating an app shows a Space with its windows.
    pub switch_on_activate: bool,
}

impl SpaceSettings {
    /// 03 §1.17's defaults.
    pub(crate) fn defaults() -> SpaceSettings {
        SpaceSettings {
            per_display: true,
            auto_rearrange: false,
            switch_on_activate: true,
        }
    }
}

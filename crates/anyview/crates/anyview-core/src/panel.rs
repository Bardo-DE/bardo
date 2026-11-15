//! What the viewer remembers of the left panel from one launch to the next.

/// The left panel as the person left it. One value for the person, not one per file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PanelKept {
    /// Whether it was open (showing, not folded away) when the person last left it.
    pub open: bool,
    /// The view it was last on, by its slug (`thumbnails`, `info`): a word this build does not
    /// know is ignored. `None` leaves the view already kept, which is how a close is kept.
    pub view: Option<String>,
}

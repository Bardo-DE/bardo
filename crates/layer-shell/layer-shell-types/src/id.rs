//! Identifiers the host hands out. Newtypes, so a surface id can never be passed where an
//! output id belongs.

use serde::{Deserialize, Serialize};

/// One surface the host manages: a layer surface, a popup or a toplevel. Never reused within a
/// process, so an id held after its surface closed simply stops matching anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SurfaceId(pub u64);

/// One `wl_output`, by the registry name the compositor gave its global. Stable for as long as
/// the output stays connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OutputId(pub u32);

/// The name a debug command addresses a surface by: the layer namespace, the toplevel app id,
/// or `<parent>/popup` for a parent's open popup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SurfaceName(pub String);

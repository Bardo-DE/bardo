//! Blitz glue for Bardo: launching an app and opening its windows, registering the faces, the
//! clipboards and the document host (`use_standalone_host`), PDF output (feature `pdf`), printing
//! (feature `print`), spellchecking (feature `spell`), the menu bar exported over D-Bus (feature
//! `menus`), and the device-pixel layout snap (`snap`). The renderer: it names the blitz crates,
//! and fills the seams of `ui-host-hooks`, which is what a UI library links instead (the document
//! host, the styles guard, the resolved tokens, the clipboard calls, the `TokioSpawner`). The test
//! driver lives in `ui-harness`, which builds its document from the parts in `seam` (feature
//! `testing`).
//!
//! It is also the only UI kit crate that may depend on `tokio` (`scripts/check-boundary.sh`
//! forbids it to `ds`): `launch` owns the process-wide runtime (`launch::runtime`), so the
//! `ui_host_hooks::TokioSpawner` it hands `ui_settings::use_environment` has a runtime to run its
//! portal and file-watch tasks on, and `enter_runtime` lends it to a test driver.

mod app_handle;
mod app_id;
mod app_life;
#[cfg(all(target_os = "linux", feature = "desktop"))]
mod bardo_window;
mod blitz_host;
mod click_focus;
pub mod clipboard;
mod close_request;
mod content_fit;
mod contexts;
#[cfg(feature = "desktop-outputs")]
mod desktop_outputs;
mod drop_hit;
mod edit;
mod edit_align;
mod edit_geometry;
mod edit_hit;
mod edit_ime;
mod edit_locate;
mod edit_tree;
mod edit_window;
pub mod error;
mod focus;
mod focus_chain;
mod focus_keep;
pub mod fonts;
mod frame_anchor;
mod frame_book;
mod frame_hit;
mod frame_hover;
mod frame_image;
mod frame_img;
mod frame_links;
mod frame_menu;
mod frame_tag;
mod frame_tree;
pub mod frames;
mod gesture_window;
mod gpu_request;
mod host;
mod install;
mod keymap_choice;
mod keymap_detect;
pub mod launch;
mod layout_chain;
mod link_open;
mod measure;
mod memory_shell;
#[cfg(all(feature = "menus", target_os = "linux"))]
pub mod menus;
mod native_providers;
mod net;
mod net_policy;
mod node_ref;
mod open_window;
mod origin;
#[cfg(feature = "pdf")]
mod pdf;
#[cfg(feature = "pdf")]
mod pdf_thumb;
mod phase;
#[cfg(feature = "print")]
mod print;
mod probe;
mod resolved;
mod reveal;
mod route;
mod scheme;
mod screen_area;
#[cfg(feature = "testing")]
pub mod seam;
mod setup;
mod shell_keys;
mod size_ledger;
mod sized_window;
pub mod snap;
#[cfg(feature = "spell")]
pub mod spell;
mod startup_token;
mod texture_layer;
#[cfg(any(test, feature = "pdf", feature = "testing"))]
mod wake;
#[cfg(all(target_os = "linux", feature = "desktop"))]
mod wayland_surface;
pub mod window;
mod window_activate;
mod window_build;
mod window_drop;
mod window_fit;
mod window_frame;
mod window_hover;
mod window_place;
mod window_platform;
mod window_requests;
mod window_screen;
mod window_scroll;
mod window_shell;
mod window_size;
mod window_sizer;
#[cfg(all(target_os = "linux", feature = "desktop"))]
mod xdg_activation;

pub use app_handle::{AppHandle, AppHold, use_app_handle};
pub use app_id::AppId;
pub use app_life::LastWindowClosed;
pub use blitz_embed::adapter::{ADAPTER_ENV, AdapterPref};
pub use blitz_embed::net::FileRoots;
pub use blitz_host::use_standalone_host;
pub use close_request::use_close_request;
pub use content_fit::{ContentFit, use_content_fit};
pub use contexts::RootContexts;
pub use error::LaunchError;
pub use fonts::font_context;
pub use frame_hover::{FrameLinkHover, HoverPhase};
pub use frame_image::FrameImageClick;
pub use frame_links::{FrameHandler, FrameLink, FrameLinks};
pub use frame_menu::FrameLinkMenu;
pub use frame_tag::FrameTag;
pub use gpu_request::GpuRequest;
pub use keymap_detect::system_key_source;
pub use launch::{AppConfig, RuntimeGuard, enter_runtime, launch, launch_idle};
pub use link_open::{LinkFailureHandler, LinkOpenError};
pub use net_policy::{AppNet, NetDecision, NetPolicy, NetReply, NetRequest};
pub use open_window::{WindowHandle, WindowSpec, open_window, open_window_with, use_window_handle};
pub use origin::FrameId;
#[cfg(feature = "pdf")]
pub use pdf::{
    ContentPx, Margins, PageSize, PageSpec, PdfError, Pt, content_size, pdf, print_document,
    print_viewport,
};
#[cfg(feature = "pdf")]
pub use pdf_thumb::{
    DeviceBox, PdfFileThumb, QUEUE_DEPTH, THUMB_CACHE_ENTRIES, ThumbKey, ThumbRequest,
    pdf_thumb_blocking, pdf_thumb_bytes, pdf_thumb_cached, pdf_thumb_rasters, use_pdf_page,
};
#[cfg(feature = "print")]
pub use print::{PrintError, PrintOutcome, print_dialog};
pub use resolved::Resolved;
pub use shell_keys::ShellKeys;
pub use snap::snap_to_device;
pub use texture_layer::{
    Gpu, GpuError, Pace, PixelFormat, Pixels, PixelsError, Sampling, TexelRect, Texels, TextureFit,
    TextureHandle, TextureLayer, use_gpu,
};
pub use ui_emoji::{EMOJI_FAMILY_ENV, EmojiChoice, colrv1_emoji_file};
pub use window::{Decorations, WinitWindow};
pub use window_scroll::{ScrollBounds, ScrollHandle, use_scroll_handle};
// What a scroll command names: the container's element `id`, the axis, and how it moves.
pub use blitz_embed::scroll::cmd::ScrollCmd;
pub use blitz_embed::scroll::engine::ScrollAnimate;
pub use blitz_embed::scroll::geom::ScrollAxis;
pub use blitz_embed_types::element_id::ElementId;
pub use screen_area::{Reserve, ScreenArea, ScreenOf, WorkBasis};
pub use size_ledger::{SizeOrigin, SizeRequest};
pub use window_fit::Fit;
pub use window_requests::WindowLife;
pub use window_size::{Extent, WindowSize};
pub use window_sizer::{WindowSizer, use_window_sizer};

// The window renderer dioxus-native runs on; named here so the pinned versions stay the ones
// the render stack resolves.
use anyrender_vello_hybrid as _;
use wgpu as _;

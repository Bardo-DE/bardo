//! Web URLs and origins, parsed once where they enter ([`WebUrl`], [`Origin`]), and
//! percent-encoding ([`percent`]). No runtime and no I/O.

mod origin;
pub mod percent;
mod weburl;

pub use origin::{Origin, UrlScheme, split_authority};
pub use weburl::WebUrl;

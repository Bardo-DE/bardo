//! Spellchecking for `ui_components::prelude::EditSurface` (the `spell` feature; design/04-COMPONENTS.md
//! section 50): `ui_components::prelude::SpellService` on Blitz. The system's Hunspell dictionaries
//! (`/usr/share/hunspell`; nothing is bundled) are read and checked by `spellbook`, unmodified
//! (MPL-2.0), on a worker thread; learned words go to `~/.local/share/bardo/spelling/<lang>.dic`.
//!
//! An app provides it at the top of its root with [`provide`] (the system's dictionaries, in
//! the locale's language); a test gives it dictionaries of its own with [`provide_with`].

mod books;
mod choose;
mod config;
mod paragraphs;
mod service;
mod worker;

pub use choose::{locale_lang, pick};
pub use config::{SYSTEM_DICTIONARIES, SpellConfig};
pub use service::NativeSpell;

use dioxus::prelude::*;
use std::rc::Rc;
use ui_components::prelude::*;
use ui_components::spell::lang::Lang;

/// Provide the system's spellchecker to the calling component's subtree.
#[must_use]
pub fn provide() -> Rc<dyn SpellService> {
    use_context_provider(|| Rc::new(NativeSpell::system()) as Rc<dyn SpellService>)
}

/// Provide a spellchecker over `config`'s dictionaries, in `languages` by default.
#[must_use]
pub fn provide_with(config: SpellConfig, languages: Vec<Lang>) -> Rc<dyn SpellService> {
    use_context_provider(|| {
        Rc::new(NativeSpell::with_config(config, languages)) as Rc<dyn SpellService>
    })
}

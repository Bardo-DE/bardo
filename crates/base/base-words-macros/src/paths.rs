//! Where the generated code finds the crate that owns a trait: the crate itself, or `ui-components`'s facade.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

/// The crate a derive names, as a path: `::base_words` where the caller depends on `base-words`
/// (or is it), else `::ui_components::words`. Each crate declares `extern crate self` under its own name.
pub(crate) fn words() -> TokenStream {
    home("base-words", "words")
}

/// The path to `ui-style`: `::ui_style` where the caller depends on it (or is it), else
/// `::ui_components::style`.
pub(crate) fn style() -> TokenStream {
    home("ui-style", "style")
}

fn home(krate: &str, facade: &str) -> TokenStream {
    if let Some(path) = named(krate) {
        return path;
    }
    match named("ui-components") {
        Some(ui) => {
            let facade = Ident::new(facade, Span::call_site());
            quote! { #ui::#facade }
        }
        None => named_default(krate),
    }
}

fn named(krate: &str) -> Option<TokenStream> {
    let found = crate_name(krate).ok()?;
    let name = match found {
        FoundCrate::Itself => krate.replace('-', "_"),
        FoundCrate::Name(name) => name,
    };
    let ident = Ident::new(&name, Span::call_site());
    Some(quote! { ::#ident })
}

/// Neither crate is a dependency: the compile error names the one a manifest lacks.
fn named_default(krate: &str) -> TokenStream {
    let ident = Ident::new(&krate.replace('-', "_"), Span::call_site());
    quote! { ::#ident }
}

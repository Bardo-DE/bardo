//! A token's resolved value for Rust geometry: a custom property as an element's computed style
//! holds it after the cascade (the quire sheet, the look, the person's own sheet), with every
//! `var()` substituted. Tokens are declared on `.ds` and inherit, so any element inside a
//! surface sees them with their overrides; a dock's gap or a menu's row height that a look may
//! move is read here, not from a settings key.
//!
//! The sheets reach the document as a `<style>` whose text follows the `Styles` signal (a look
//! or a style sheet change restyles the document on the next frame). A component that sizes
//! itself from tokens uses [`use_resolved`]: the renderer reads them after every resolved frame,
//! outside render, and sets the component's signal only when the reading changed, so a restyle
//! re-renders it with the new values and nothing waits on a timer.
//!
//! [`Tokens`] reads on demand instead, for a read in an event handler or a task. It answers
//! `None` while the document is borrowed, which is always the case during a component's render.
//!
//! The document is the renderer's: this crate reads it through the [`TokenSource`] the renderer
//! provides as a root context, and a root without one reads nothing.

use std::any::Any;
use std::rc::Rc;

use dioxus::prelude::*;
use ui_components::prelude::Px;
use ui_components::style::task::try_set;

/// What the renderer can say of one element's resolved custom properties.
pub trait TokenReader {
    /// The computed text of custom property `name` (`--gap`), with every `var()` substituted and
    /// trimmed; `None` when the element has none, or `name` is not a custom property.
    fn text(&self, name: &str) -> Option<String>;
}

/// A changed reading handed back to its owner, to run where the owner's signal lives; `false`
/// once the owner is gone.
pub type Publish = Box<dyn FnOnce() -> bool>;

/// A reading of one element, given the resolved document. It keeps what it last read and answers
/// with the publish to run only when that changed.
pub type Reading = Box<dyn FnMut(&dyn TokenReader) -> Option<Publish>>;

/// The renderer's side of [`Tokens`] and [`use_resolved`], provided as `Rc<dyn TokenSource>` in
/// the root context of every document it hosts.
pub trait TokenSource {
    /// Ask `ask` about the element mounted as `element`, now. `ask` is not called while the
    /// document is borrowed, or when `element` is not one of the renderer's.
    fn read(&self, element: &MountedData, ask: &mut dyn FnMut(&dyn TokenReader));

    /// Run `reading` on `element` after every resolved frame until the returned registration is
    /// dropped; `None` when this document has no resolve step to follow or `element` is not one of
    /// the renderer's.
    #[must_use = "the reading ends when the registration is dropped"]
    fn watch(&self, element: &MountedData, reading: Reading) -> Option<Box<dyn Any>>;
}

/// A custom property's text as a length in px (`12px`), or `None`.
fn length_of(text: &str) -> Option<Px> {
    parse_number(text.strip_suffix("px")?).map(Px)
}

fn parse_number(text: &str) -> Option<f32> {
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

/// What a reading sees: the resolved custom properties of one element.
#[derive(Clone, Copy)]
pub struct Lookup<'a> {
    reader: &'a dyn TokenReader,
}

impl Lookup<'_> {
    /// The custom property `name` (`--gap`) as a length in px; `None` when it is not a `px`
    /// length.
    #[must_use]
    pub fn length(&self, name: &str) -> Option<Px> {
        length_of(&self.reader.text(name)?)
    }

    /// The custom property `name` as a number (a unitless token); `None` when it is not one.
    #[must_use]
    pub fn number(&self, name: &str) -> Option<f32> {
        parse_number(&self.reader.text(name)?)
    }
}

impl std::fmt::Debug for Lookup<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lookup").finish_non_exhaustive()
    }
}

/// On-demand reads of the tokens at one element. Made by [`use_tokens`]; `mounted` is the
/// `onmounted` of the element they are read at.
#[derive(Clone, Copy)]
pub struct Tokens {
    element: Signal<Option<Rc<MountedData>>>,
    source: CopyValue<Option<Rc<dyn TokenSource>>>,
}

impl PartialEq for Tokens {
    fn eq(&self, other: &Self) -> bool {
        self.element == other.element
    }
}

impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tokens").finish_non_exhaustive()
    }
}

/// Reads of the resolved tokens at an element, in a handler or a task.
#[must_use]
pub fn use_tokens() -> Tokens {
    Tokens {
        element: use_signal(|| None),
        source: use_hook(|| CopyValue::new(try_consume_context::<Rc<dyn TokenSource>>())),
    }
}

impl Tokens {
    /// The `onmounted` handler of the element the tokens are read at.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "`onmounted` handlers hand over an owned event, and callers across the shell pass it straight on"
    )]
    pub fn mounted(mut self, event: MountedEvent) {
        self.element.set(Some(event.data()));
    }

    /// The custom property `name` as a length in px, `None` before the element mounts, while the
    /// document is borrowed, or when it is not a `px` length.
    #[must_use]
    pub fn length(&self, name: &str) -> Option<Px> {
        self.read(|lookup| lookup.length(name))
    }

    /// The custom property `name` as a number, `None` as for [`Tokens::length`].
    #[must_use]
    pub fn number(&self, name: &str) -> Option<f32> {
        self.read(|lookup| lookup.number(name))
    }

    fn read<T>(&self, answer: impl FnOnce(&Lookup<'_>) -> Option<T>) -> Option<T> {
        let element = self.element.peek().clone()?;
        let source = self.source.peek().clone()?;
        let mut answer = Some(answer);
        let mut found = None;
        source.read(&element, &mut |reader| {
            if let Some(answer) = answer.take() {
                found = answer(&Lookup { reader });
            }
        });
        found
    }
}

/// A token reading that follows the document: the value last read after a resolved frame. Made by
/// [`use_resolved`].
pub struct ResolvedTokens<T: 'static> {
    value: Signal<Option<T>>,
    registration: Signal<Option<Box<dyn Any>>>,
    source: CopyValue<Option<Rc<dyn TokenSource>>>,
    read: fn(&Lookup) -> T,
}

impl<T> Clone for ResolvedTokens<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ResolvedTokens<T> {}

impl<T> PartialEq for ResolvedTokens<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> std::fmt::Debug for ResolvedTokens<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedTokens").finish_non_exhaustive()
    }
}

/// Tokens read with `read` at an element and read again after every resolved frame: the value is
/// `None` until the element mounts and the first frame resolves.
pub fn use_resolved<T: PartialEq + Clone + 'static>(read: fn(&Lookup) -> T) -> ResolvedTokens<T> {
    ResolvedTokens {
        value: use_signal(|| None),
        registration: use_signal(|| None),
        source: use_hook(|| CopyValue::new(try_consume_context::<Rc<dyn TokenSource>>())),
        read,
    }
}

impl<T: PartialEq + Clone + 'static> ResolvedTokens<T> {
    /// The latest reading, `None` until the first.
    #[must_use]
    pub fn value(&self) -> ReadSignal<Option<T>> {
        self.value.into()
    }

    /// The `onmounted` handler of the element the tokens are read at: starts the reading.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "`onmounted` handlers hand over an owned event, and callers across the shell pass it straight on"
    )]
    pub fn mounted(mut self, event: MountedEvent) {
        let Some(source) = self.source.peek().clone() else {
            return;
        };
        let (sink, read) = (self.value, self.read);
        let mut last: Option<T> = None;
        let registration = source.watch(
            &event.data(),
            Box::new(move |reader| {
                let now = read(&Lookup { reader });
                if last.as_ref() == Some(&now) {
                    return None;
                }
                last = Some(now.clone());
                Some(Box::new(move || try_set(sink, Some(now)).is_ok()))
            }),
        );
        self.registration.set(registration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed;

    impl TokenReader for Fixed {
        fn text(&self, name: &str) -> Option<String> {
            match name {
                "--gap" => Some("12px".to_owned()),
                "--ratio" => Some("1.5".to_owned()),
                "--bad" => Some("calc(1px + 2px)".to_owned()),
                _ => None,
            }
        }
    }

    #[test]
    fn a_lookup_reads_px_lengths_and_numbers_and_nothing_else() {
        let lookup = Lookup { reader: &Fixed };
        assert_eq!(lookup.length("--gap"), Some(Px(12.0)));
        assert_eq!(lookup.length("--ratio"), None, "a number is not a length");
        assert_eq!(lookup.length("--bad"), None);
        assert_eq!(lookup.number("--ratio"), Some(1.5));
        assert_eq!(lookup.number("--gap"), None, "a length is not a number");
        assert_eq!(lookup.length("--missing"), None);
    }
}

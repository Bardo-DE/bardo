//! A token's resolved value for Rust geometry: a custom property as an element's computed style
//! holds it after the cascade, with every `var()` substituted, read from a Blitz document. The
//! hooks that components call (`use_resolved`, `use_tokens`) are `ui_host_hooks`'s; this is the
//! document side they read through: [`Resolved`] on a node, and the [`TokenSource`] this crate
//! provides to every document it hosts.

use blitz_dom::{Atom, BaseDocument, NodeId};
use blitz_embed::after_resolve::AfterResolve;
use dioxus::prelude::MountedData;
use dioxus_native_dom::NodeHandle;
use std::any::Any;
use std::rc::Rc;
use ui_components::prelude::Px;
use ui_host_hooks::{Reading, TokenReader, TokenSource};

/// Reads of resolved custom properties on one node of a document.
#[derive(Debug)]
pub struct Resolved;

impl Resolved {
    /// `name` (`--dock-gap`) on `node` as a length in logical pixels: `Some` only for a plain
    /// `<number>px` value, `None` for a missing property, another unit, or anything not a single
    /// length (a token written as `calc()` is not evaluated, so a geometry token is a plain
    /// length).
    pub fn length(doc: &BaseDocument, node: NodeId, name: &str) -> Option<Px> {
        let text = Self::text(doc, node, name)?;
        let number = text.strip_suffix("px")?;
        parse_number(number).map(Px)
    }

    /// `name` (`--ratio`) on `node` as a plain `<number>`: `None` for a missing property or a
    /// value with a unit. A duration is not a number here.
    pub fn number(doc: &BaseDocument, node: NodeId, name: &str) -> Option<f32> {
        parse_number(&Self::text(doc, node, name)?)
    }

    /// The substituted text of the property on `node`, trimmed.
    fn text(doc: &BaseDocument, node: NodeId, name: &str) -> Option<String> {
        let key = Atom::from(name.strip_prefix("--").filter(|rest| !rest.is_empty())?);
        let styles = doc.get_node(node)?.primary_styles()?;
        let properties = styles.custom_properties();
        let value = properties
            .inherited
            .get(&key)
            .or_else(|| properties.non_inherited.get(&key))?;
        Some(value.as_universal()?.css_text().trim().to_owned())
    }
}

/// One node's resolved properties, for a reading that holds the document.
struct NodeTokens<'a> {
    doc: &'a BaseDocument,
    node: NodeId,
}

impl TokenReader for NodeTokens<'_> {
    fn text(&self, name: &str) -> Option<String> {
        Resolved::text(self.doc, self.node, name)
    }
}

/// A document's [`TokenSource`]: reads on demand through the element's node, and follows the
/// document's resolved frames through its [`AfterResolve`] when it has one.
pub(crate) struct BlitzTokens {
    after: Option<AfterResolve>,
}

impl BlitzTokens {
    /// The source of a document whose resolved frames `after` reports (`None` for a root that is
    /// not hosted by a document with a resolve step).
    pub(crate) fn new(after: Option<AfterResolve>) -> Rc<dyn TokenSource> {
        Rc::new(BlitzTokens { after })
    }
}

impl TokenSource for BlitzTokens {
    fn read(&self, element: &MountedData, ask: &mut dyn FnMut(&dyn TokenReader)) {
        let Some(handle) = element.downcast::<NodeHandle>() else {
            return;
        };
        let Some(doc) = handle.try_doc() else {
            return;
        };
        ask(&NodeTokens {
            doc: &doc,
            node: handle.node_id(),
        });
    }

    fn watch(&self, element: &MountedData, mut reading: Reading) -> Option<Box<dyn Any>> {
        let after = self.after.as_ref()?;
        let node = element.downcast::<NodeHandle>().map(NodeHandle::node_id)?;
        let registration = after.watch(
            node,
            Box::new(move |doc, node| reading(&NodeTokens { doc, node })),
        );
        Some(Box::new(registration))
    }
}

fn parse_number(text: &str) -> Option<f32> {
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

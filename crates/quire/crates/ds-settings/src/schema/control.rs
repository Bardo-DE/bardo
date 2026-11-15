//! The controls a key can be drawn with, and the one place that says which a key gets
//! (design/22-SETTINGS.md section 9.1). Every program that draws settings reads this table; none
//! keeps its own.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

use super::key::{KeyKind, KeySpec};

/// The id of a control, as a schema writes it: `control = "picture"` on a key. Kept as text so a
/// schema that names a control this build does not have still reads; [`Control::named`] says
/// whether it is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ControlName(pub String);

/// One control of the Settings vocabulary. The set is closed and owned by Settings: an owner
/// picks one by id, it never ships its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum Control {
    /// A read-only label: the one value a one-variant enum can hold.
    Readout,
    /// A switch, for two words that are on and off.
    Toggle,
    /// Segments side by side, for a choice between two to five words.
    Segmented,
    /// A pop-up button, for any choice.
    Menu,
    /// A slider, for a bounded number.
    Slider,
    /// A text field.
    Text,
    /// A colour swatch row.
    Colour,
    /// A captured key chord.
    Shortcut,
    /// A rows editor, one cell per column.
    Rows,
    /// A button that runs a live module's action.
    Button,
    /// A grid of pictures with "Add Picture…"; the value is the path of the chosen one. A text
    /// key only.
    Picture,
    /// The four hot corners as one sheet: a screen picture with a menu at each corner. A text
    /// key only; the value is the corner's list of bindings.
    Corners,
}

impl Control {
    /// The control called `name`, `None` for one this build does not have. A reader then draws
    /// the key's default control.
    pub fn named(name: &ControlName) -> Option<Control> {
        Control::parse(&name.0)
    }

    /// This control's id as a schema writes it.
    pub fn name(self) -> ControlName {
        ControlName(self.slug().to_owned())
    }

    /// Whether this control can draw a key of `kind`. A control that cannot is ignored, so a
    /// schema never makes a slider of a word.
    pub fn fits(self, kind: &KeyKind) -> bool {
        match self {
            Control::Picture | Control::Corners => matches!(kind, KeyKind::Text),
            Control::Menu => matches!(
                kind,
                KeyKind::Segmented { .. }
                    | KeyKind::Menu { .. }
                    | KeyKind::Provided { .. }
            ),
            other => other == kind.default_control(),
        }
    }
}

impl KeyKind {
    /// The control a key of this kind gets when its schema names none. Total: every kind is
    /// written, so a new [`KeyKind`] is a compile error here until it picks one.
    pub fn default_control(&self) -> Control {
        match self {
            KeyKind::Fixed { .. } => Control::Readout,
            KeyKind::Toggle => Control::Toggle,
            KeyKind::Segmented { .. } => Control::Segmented,
            KeyKind::Menu { .. } | KeyKind::Provided { .. } => Control::Menu,
            KeyKind::Bounded { .. } => Control::Slider,
            KeyKind::Text => Control::Text,
            KeyKind::Colour => Control::Colour,
            KeyKind::Shortcut => Control::Shortcut,
            KeyKind::List(_) | KeyKind::Rows { .. } => Control::Rows,
            KeyKind::Live { .. } => Control::Button,
        }
    }
}

impl KeySpec {
    /// The control that draws this key: the one its schema names when this build has it and it
    /// fits the key's kind, else the kind's default.
    pub fn control(&self) -> Control {
        self.control
            .as_ref()
            .and_then(Control::named)
            .filter(|control| control.fits(&self.kind))
            .unwrap_or_else(|| self.kind.default_control())
    }
}

#[cfg(test)]
mod tests {
    use super::{Control, ControlName};
    use crate::schema::{
        ActionLabel, ActionWeight, Exposure, Help, KeyKind, KeyPath, KeySpec, Label, LiveAction,
        Page, Section,
    };

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|word| (*word).to_owned()).collect()
    }

    fn key(kind: KeyKind, control: Option<&str>) -> KeySpec {
        KeySpec {
            ends: None,
            path: KeyPath("a.b".to_owned()),
            kind,
            default: toml::Value::String(String::new()),
            label: Label("B".to_owned()),
            help: Help(String::new()),
            page: Page::Appearance,
            section: Section(String::new()),
            exposure: Exposure::Basic,
            labels: Default::default(),
            agent: Default::default(),
            unavailable: Default::default(),
            groups: Default::default(),
            places: Default::default(),
            note: None,
            control: control.map(|name| ControlName(name.to_owned())),
        }
    }

    #[test]
    fn each_kind_gets_its_default_control() {
        let bounded = KeyKind::Bounded {
            min: 0,
            max: 9,
            unit: None,
        };
        let live = KeyKind::Live {
            action: LiveAction {
                label: ActionLabel("Revoke".to_owned()),
                weight: ActionWeight::Plain,
            },
        };
        let cases = [
            (
                KeyKind::Fixed {
                    variant: "a".to_owned(),
                },
                Control::Readout,
            ),
            (KeyKind::Toggle, Control::Toggle),
            (
                KeyKind::Segmented {
                    variants: words(&["a", "b", "c"]),
                },
                Control::Segmented,
            ),
            (
                KeyKind::Menu {
                    variants: words(&["a", "b"]),
                },
                Control::Menu,
            ),
            (bounded, Control::Slider),
            (
                KeyKind::Provided {
                    capability: "look".to_owned(),
                },
                Control::Menu,
            ),
            (KeyKind::Text, Control::Text),
            (KeyKind::Colour, Control::Colour),
            (KeyKind::Shortcut, Control::Shortcut),
            (KeyKind::List(Box::new(KeyKind::Text)), Control::Rows),
            (KeyKind::Rows { columns: vec![] }, Control::Rows),
            (live, Control::Button),
        ];
        for (kind, want) in cases {
            assert_eq!(kind.default_control(), want, "{kind:?}");
        }
    }

    #[test]
    fn a_named_control_is_used_when_it_exists_and_fits_else_the_default_stands() {
        let menu = KeyKind::Segmented {
            variants: words(&["a", "b", "c"]),
        };
        let cases = [
            (
                "picture on text",
                KeyKind::Text,
                Some("picture"),
                Control::Picture,
            ),
            (
                "picture on a slider",
                KeyKind::Bounded {
                    min: 0,
                    max: 9,
                    unit: None,
                },
                Some("picture"),
                Control::Slider,
            ),
            (
                "corners on text",
                KeyKind::Text,
                Some("corners"),
                Control::Corners,
            ),
            (
                "a menu for segments",
                menu.clone(),
                Some("menu"),
                Control::Menu,
            ),
            (
                "a slider on text",
                KeyKind::Text,
                Some("slider"),
                Control::Text,
            ),
            (
                "a control nobody has",
                KeyKind::Text,
                Some("hologram"),
                Control::Text,
            ),
            ("none named", menu, None, Control::Segmented),
        ];
        for (name, kind, control, want) in cases {
            assert_eq!(key(kind, control).control(), want, "{name}");
        }
    }

    #[test]
    fn a_control_name_round_trips_through_its_id() {
        for control in <Control as ds_core::word::Word>::ALL {
            assert_eq!(Control::named(&control.name()), Some(*control));
        }
    }
}

//! Sidebar with several sections and a foot: pinned tiles, two source lists and a foot of
//! Space dots, on its own ground and clear over a Space's tint.

use crate::pages::{Section, Specimen};
use dioxus::prelude::*;
use ui_components::base::vocab::RowState;
use ui_components::components::app::pin_tile::PinFace;
use ui_components::components::app::pin_tiles::{PinItem, PinTiles};
use ui_components::components::chrome::sidebar::Sidebar;
use ui_components::components::chrome::sidebar_model::{SidebarFill, SidebarSection};
use ui_components::components::content::provider_mark::MarkProvider;
use ui_components::components::controls::button_model::{Bezel, ImagePosition};
use ui_components::prelude::*;
use ui_components::style::space::frame_vars::FrameVars;
use ui_components::style::space::presets::default_look;
use ui_components::style::tokens::control_size::ControlSize;
use ui_components::style::tokens::hex::{Colour, Hex};

/// A list of places under its own heading.
fn places(
    heading: &'static str,
    rows: &'static [(&'static str, &'static str, Icon)],
    here: &'static str,
    onselect: EventHandler<&'static str>,
) -> SidebarSection<&'static str> {
    let mut items = vec![ListItem::heading(
        heading,
        rsx! { SectionHeader { title: heading } },
    )];
    items.extend(rows.iter().map(|&(key, title, icon)| {
        ListItem::row(
            key,
            title,
            rsx! {
                Row {
                    title: TextLine::from(title),
                    leading: RowLeading::Icon(icon),
                    state: RowState {
                        selection: here == key,
                        ..RowState::default()
                    },
                    onclick: move |_| onselect.call(key),
                }
            },
        )
    }));
    SidebarSection::List(items)
}

const PLACES: &[(&str, &str, Icon)] = &[
    ("inbox", "Inbox", Icon::Inbox),
    ("sent", "Sent", Icon::Send),
    ("trash", "Trash", Icon::Trash),
];

const VIEWS: &[(&str, &str, Icon)] = &[
    ("thumbnails", "Thumbnails", Icon::Inbox),
    ("contents", "Contents", Icon::Tag),
    ("info", "Info", Icon::Send),
];

const LABELS: &[(&str, &str, Icon)] =
    &[("work", "Work", Icon::Tag), ("travel", "Travel", Icon::Tag)];

/// The Space dots, a plus and the hide button.
fn foot(scheme: Scheme) -> Element {
    let dot = |index: usize, name: &'static str, selection: bool| {
        let look = default_look(index, Grain(0));
        rsx! {
            SpaceDot {
                key: "{index}",
                name,
                face: DotFace::Foot,
                frame: FrameVars::of(&look, scheme),
                selection,
                shortcut: Shortcut(vec![]),
                onclick: |()| {},
            }
        }
    };
    rsx! {
        Button { bezel: Bezel::Inline, label: "Home", title: "Edit this Space".to_owned(), onclick: |_| {} }
        div { style: "display:flex;gap:var(--s-5);margin-left:auto",
            {dot(0, "Home", true)}
            {dot(2, "Work", false)}
        }
        Button {
            bezel: Bezel::Toolbar, image: ImagePosition::Only, size: ControlSize::Small,
            icon: Icon::Plus, label: "New Space", onclick: |_| {},
        }
    }
}

/// The Sidebar sections specimen.
#[component]
pub fn SidebarSectionsSection() -> Element {
    let mut here = use_signal(|| "inbox");
    let scheme = use_scope().scheme;
    let onselect = EventHandler::new(move |key: &'static str| here.set(key));
    let tiles = vec![
        PinItem::new(
            'P',
            PinFace::Account {
                initial: 'P',
                colour: Colour::Solid(Hex([0x1a, 0x73, 0xe8])),
                provider: MarkProvider::Google,
                address: Some("p@example.org".to_owned()),
            },
        )
        .unread(3),
    ];
    rsx! {
        Section { title: "Sidebar: sections and a foot", note: "Sections of source-list Lists (their headings are ListItem::heading) and Custom content (pinned tiles) scroll between the header and the foot, which stays at the bottom; one cursor runs across every list. The default fill is the sidebar's own ground; SidebarFill::Clear paints nothing, so the window's colour (a Space's flat tint) shows through, with the ordinary inks; SidebarSection::Fill takes the height the rows above leave and gives its content that height (a list that scrolls itself, as Preview's thumbnails do), and the body no longer scrolls as a whole; SidebarFill::Flat paints --surface-2, one lightness step above the paper a content-first app (terminal, editor) draws its content on: one tone, no gradient, no Space hue. A selected sidebar row is the neutral --sel-neutral wash with the accent on its symbol.",
            div { class: "g-row g-row-top",
                for (name , fill) in [("SidebarFill::Material", SidebarFill::Material), ("SidebarFill::Clear", SidebarFill::Clear), ("SidebarFill::Flat", SidebarFill::Flat)] {
                    Specimen { key: "{name}", name,
                        div { class: if fill == SidebarFill::Clear { "g-side-frame g-on-tint" } else { "g-side-frame" },
                            Sidebar::<&'static str> {
                                label: "Mail",
                                fill,
                                cursor: Some(here()),
                                onselect,
                                sections: vec![
                                    SidebarSection::Custom(rsx! {
                                        PinTiles::<char> { label: "Accounts", items: tiles.clone(), selected: Some('P'), onpick: |_| {} }
                                    }),
                                    places("Places", PLACES, here(), onselect),
                                    places("Labels", LABELS, here(), onselect),
                                ],
                                foot: foot(scheme),
                            }
                        }
                    }
                }
                Specimen { name: "SidebarSection::Fill",
                    div { class: "g-side-frame",
                        Sidebar::<&'static str> {
                            label: "Viewer",
                            cursor: Some("thumbnails"),
                            onselect,
                            sections: vec![
                                places("View", VIEWS, "thumbnails", onselect),
                                SidebarSection::Fill(rsx! {
                                    div { style: "overflow-y:auto",
                                        for page in 1..=40 {
                                            Row { key: "{page}", title: TextLine::from(format!("Page {page}")) }
                                        }
                                    }
                                }),
                            ],
                        }
                    }
                }
            }
        }
    }
}

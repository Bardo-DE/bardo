//! Where the keyboard goes: kept when a click lands on nothing focusable, moved by a click a
//! control keeps to itself, handed on when the focused element leaves, set by a field's handle or a
//! selector, restored after a click removed its target, and selected-all on a controlled focus.
//! One rule (the focus fallback keeps the keyboard in the app), one module per scenario.

mod keep {
    //! Native focus, keep-focus: a click on nothing focusable leaves the keyboard on the nearest
    //! focusable ancestor (mailo's `.app[tabindex]`), as a browser does, where Blitz would clear it;
    //! `FocusFallback::BlitzDefault` keeps Blitz's behaviour. A field the click leaves still hears
    //! its blur, the ancestor hears no spurious one, a double click still arrives, and an edit
    //! surface's own focus wins inside it.

    use dioxus::prelude::*;
    use std::time::Duration;
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use ui_host_hooks::FocusFallback;

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 320,
        scale_percent: 100,
    };

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// A key-handling shell around plain text, a field and an edit surface, logging what it hears.
    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn Shell() -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let mut note = move |line: String| log.with_mut(|log| log.push(line));
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window,
                div { class: "app", tabindex: "0", style: "padding:20px; width:300px",
                    onkeydown: move |event: KeyboardEvent| note(format!("key:{}", event.key())),
                    onblur: move |_| note("app-blur".to_owned()),
                    p { class: "plain", style: "margin:0; height:24px",
                        ondoubleclick: move |_| note("dblclick".to_owned()),
                        "Plain text, nothing focusable"
                    }
                    div { class: "field", style: "display:flex; margin-top:12px",
                        TextField { label: "Find", value: "",
                            oninput: |_| {},
                            onblur: move |()| note("field-blur".to_owned()),
                        }
                    }
                    div { style: "margin-top:12px; height:40px",
                        EditSurface { common: Common { id: Some("editor".to_string()), ..Common::default() }, on_input: |_| {},
                            p { class: "inside", style: "margin:0", "Editable text" }
                        }
                    }
                }
                p { class: "log", {log().join(",")} }
            }
        }
    }

    fn harness(fallback: FocusFallback) -> Harness {
        let mut harness = Harness::new(
            Shell,
            HarnessConfig::new(VIEW)
                .with_clock(Clock::Virtual)
                .with_focus_fallback(fallback),
        );
        harness.advance(ms(50));
        harness
    }

    fn log(harness: &Harness) -> String {
        harness.text_of(".log").unwrap_or_default()
    }

    fn click(harness: &mut Harness, selector: &str) {
        let at = harness
            .centre(selector)
            .unwrap_or_else(|| panic!("{selector} is not in the document:\n{}", harness.html()));
        harness.send(Input::click(at));
        // A frame: the fallback waits out a document busy with the click's own re-render.
        harness.advance(ms(60));
    }

    #[test]
    fn a_click_on_plain_text_leaves_the_shell_focused_and_it_hears_the_next_key() {
        let mut harness = harness(FocusFallback::Ancestor);
        click(&mut harness, ".plain");
        assert!(harness.focus_of(".app"));
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(log(&harness), "key:j");
    }

    #[test]
    fn with_blitz_default_the_click_clears_the_focus() {
        let mut harness = harness(FocusFallback::BlitzDefault);
        click(&mut harness, ".plain");
        assert!(!harness.focus_of(".app"));
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(log(&harness), "");
    }

    #[test]
    fn a_double_click_still_arrives() {
        let mut harness = harness(FocusFallback::Ancestor);
        click(&mut harness, ".plain");
        click(&mut harness, ".plain");
        assert!(harness.focus_of(".app"));
        assert_eq!(log(&harness), "dblclick");
    }

    #[test]
    fn a_field_the_click_leaves_hears_its_blur_and_the_shell_takes_the_keyboard() {
        let mut harness = harness(FocusFallback::Ancestor);
        click(&mut harness, ".field input");
        assert!(harness.focus_of(".field input"));
        click(&mut harness, ".plain");
        assert!(harness.focus_of(".app"));
        assert_eq!(log(&harness), "field-blur");
    }

    #[test]
    fn inside_an_edit_surface_its_own_focus_wins() {
        let mut harness = harness(FocusFallback::Ancestor);
        click(&mut harness, ".plain");
        click(&mut harness, ".inside");
        assert!(harness.focus_of("#editor"));
        assert!(!harness.focus_of(".app"));
    }
}

mod kept_click {
    //! A click a UI kit control keeps to itself still moves the keyboard (mailo, against v0.1.10).
    //!
    //! A strip button, a row's disclosure triangle and anything in its trailing slot stop their
    //! click, so it never reached `Ds`'s click-focus fallback; Blitz then cleared the focus (or,
    //! with the default prevented, left it where it was), and the keys went to `html`. Each control
    //! now hands its kept click to the host itself: under `FocusFallback::Ancestor` the pressed
    //! control (or its nearest focusable ancestor) has the keyboard afterwards, as a browser leaves
    //! it. Under `FocusFallback::BlitzDefault` nothing changes: the negative control.

    use dioxus::prelude::*;
    use std::time::Duration;
    use ui_components::base::press::Press;
    use ui_components::components::app::hover_strip::{ActionId, HoverStrip, StripAction};
    use ui_components::components::controls::button_model::{Bezel, ImagePosition};
    use ui_components::components::lists::row::row::Outline;
    use ui_components::focus::request::use_focus_request;
    use ui_components::prelude::*;
    use ui_harness::harness::settle_until;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use ui_host_hooks::FocusFallback;

    const VIEW: Viewport = Viewport {
        width: 480,
        height: 320,
        scale_percent: 100,
    };

    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn Page() -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let mut note = move |entry: &str| log.with_mut(|log| log.push(entry.to_owned()));
        let actions = vec![StripAction {
            id: ActionId("archive".to_string()),
            icon: Icon::Archive,
            label: "Archive".to_string(),
            fly: "Archive → out of Inbox".to_string(),
            onhover: None,
            onclick: EventHandler::new(|_| {}),
        }];
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window,
                div { class: "app", tabindex: "0", style: "padding:20px; width:400px",
                    onkeydown: move |event: KeyboardEvent| note(&format!("key:{}", event.key())),
                    div { class: "row", style: "display:flex; height:40px",
                        onclick: move |_| note("open"),
                        HoverStrip {
                            actions,
                            shown: true,
                            on_press: move |id: ActionId| note(&format!("press:{}", id.0)),
                        }
                    }
                    div { class: "tree", style: "width:260px",
                        Row {
                            title: "Projects",
                            leading: RowLeading::Icon(Icon::Folder),
                            outline: Outline::Branch(true),
                            on_toggle: move |_| note("toggle:projects"),
                            onclick: move |_: Press| note("select"),
                            accessory: Accessory::Slot(rsx! {
                                Button {
                                    bezel: Bezel::Toolbar, image: ImagePosition::Only,
                                    icon: Icon::Ellipsis,
                                    label: "More",
                                    onclick: move |_: Press| note("more"),
                                }
                            }),
                            Row { title: "Bardo", outline: Outline::Leaf }
                        }
                    }
                    div { class: "blank", style: "height:20px" }
                    p { class: "log", {log().join(",")} }
                }
            }
        }
    }

    fn harness(fallback: FocusFallback) -> Harness {
        let mut harness = Harness::new(
            Page,
            HarnessConfig::new(VIEW)
                .with_clock(Clock::Virtual)
                .with_focus_fallback(fallback),
        );
        harness.advance(Duration::from_millis(50));
        harness
    }

    /// Click the element `selector` names, at its centre.
    fn click(harness: &mut Harness, selector: &str) {
        let at = harness
            .centre(selector)
            .unwrap_or_else(|| panic!("{selector} is not laid out:\n{}", harness.html()));
        harness.send(Input::click(at));
    }

    const STRIP_BUTTON: &str = ".row .ds-strip .ds-button";
    const TRIANGLE: &str = ".tree .ds-row-disclosure[*|data-outline=branch]";
    const TRAILING: &str = ".tree .ds-row-trailing .ds-button";

    /// The press was the control's own (the log says so), and then the keyboard is on `focused`
    /// and nowhere else: a key reaches `.app` through it.
    fn pressed_focuses(selector: &str, logged: &str, focused: &str) {
        let mut harness = harness(FocusFallback::Ancestor);
        click(&mut harness, selector);
        settle_until(&mut harness, |harness| harness.focus_of(focused));
        assert!(!harness.focus_of("html"), "the keyboard is not on html");
        harness.advance(Duration::from_millis(100));
        assert!(harness.focus_of(focused), "{focused} kept the keyboard");
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(
            harness.text_of(".log").as_deref(),
            Some(format!("{logged},key:j").as_str()),
            "the press stayed the control's and the key reached .app"
        );
    }

    #[test]
    fn a_kept_click_focuses_the_control_it_pressed() {
        // The strip button, the row's disclosure triangle and a control in its trailing slot.
        let cases = [
            (STRIP_BUTTON, "press:archive"),
            (TRIANGLE, "toggle:projects"),
            (TRAILING, "more"),
        ];
        for (selector, logged) in cases {
            pressed_focuses(selector, logged, selector);
        }
    }

    /// The negative control: under Blitz's own behaviour a kept click leaves the keyboard nowhere,
    /// as it did for every one of them before.
    #[test]
    fn under_blitz_default_a_kept_click_leaves_the_keyboard_nowhere() {
        for selector in [STRIP_BUTTON, TRIANGLE, TRAILING] {
            let mut harness = harness(FocusFallback::BlitzDefault);
            click(&mut harness, selector);
            harness.advance(Duration::from_millis(200));
            assert!(
                !harness.focus_of(selector) && !harness.focus_of(".app"),
                "{selector}: Blitz's own default gives no control the keyboard"
            );
        }
    }

    /// One folder being renamed, another beside it; `.app` logs its pointer-ups, as mailo's ends a
    /// drag there.
    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn Renaming() -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let mut note = move |entry: &str| log.with_mut(|log| log.push(entry.to_owned()));
        let request = use_focus_request().with_select_all();
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window,
                div { class: "app", tabindex: "0", style: "padding:20px; width:300px",
                    onpointerup: move |_| note("up"),
                    div { class: "tree", style: "width:260px",
                        Row {
                            title: "Projects",
                            content: rsx! {
                                span { onclick: move |event: MouseEvent| event.stop_propagation(),
                                    TextField { bezeled: false, label: "Rename folder", value: "Projects",
                                        focus: FieldFocus::Controlled(request),
                                        oninput: |_: String| {},
                                    }
                                }
                            },
                            leading: RowLeading::Icon(Icon::Folder),
                            outline: Outline::Branch(false),
                            on_toggle: move |_| note("toggle:projects"),
                            span { "inside" }
                        }
                        Row {
                            title: "Archive",
                            outline: Outline::Branch(false),
                            on_toggle: move |_| note("toggle:archive"),
                            onclick: move |_: Press| note("select"),
                            span { "inside" }
                        }
                    }
                    p { class: "log", {log().join(",")} }
                }
            }
        }
    }

    const FIELD: &str = ".ds-row-words input";

    fn renaming() -> Harness {
        let mut harness = Harness::new(
            Renaming,
            HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
        );
        settle_until(&mut harness, |harness| harness.focus_of(FIELD));
        harness
    }

    /// A press in the editing slot's field reaches an ancestor's pointer-up (the slot no longer
    /// fences it), and still neither toggles nor selects the row.
    #[test]
    fn a_press_in_the_rename_field_reaches_the_apps_pointerup() {
        let mut harness = renaming();
        click(&mut harness, FIELD);
        harness.advance(Duration::from_millis(100));
        assert_eq!(harness.text_of(".log").as_deref(), Some("up"));
        assert!(harness.focus_of(FIELD), "the field kept the keyboard");
    }

    /// A kept click with its default prevented cannot blur a field, so a field that has the
    /// keyboard keeps it: the rename is not left without its field hearing so.
    #[test]
    fn a_field_with_the_keyboard_keeps_it_through_a_kept_click() {
        let mut harness = renaming();
        click(
            &mut harness,
            ".tree > .ds-row-branch:nth-child(2) .ds-row-disclosure",
        );
        harness.advance(Duration::from_millis(200));
        assert_eq!(
            harness.text_of(".log").as_deref(),
            Some("up,toggle:archive")
        );
        assert!(harness.focus_of(FIELD), "the field kept the keyboard");
    }
}

mod removed {
    //! The keyboard after the element that had it leaves the document. Blitz
    //! resets the focus to nowhere when the focused node is removed, so after a Bardo Menu closed on
    //! Escape (its panel had the keyboard) or a field removed itself on Enter, keys reached nothing
    //! and mailo's `.app[tabindex]` stopped hearing them. Under `FocusFallback::Ancestor` the host
    //! now focuses the removed element's nearest focusable ancestor, or (for a floating menu's
    //! panel, whose ancestors are the overlay layer) the element focused before it; a menu anchored
    //! to a mounted element gives the keyboard back to that element itself.

    use dioxus::prelude::*;
    use std::time::Duration;
    use ui_components::base::press::Press;
    use ui_components::host::measure::{Anchor, MountedRef};
    use ui_components::prelude::*;
    use ui_harness::harness::settle_until;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use ui_host_hooks::FocusFallback;

    const VIEW: Viewport = Viewport {
        width: 480,
        height: 320,
        scale_percent: 100,
    };

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn entries() -> Vec<MenuItem<u8>> {
        vec![MenuItem::new(0, "Rename"), MenuItem::new(1, "Delete")]
    }

    /// A shell whose keys open a menu ("m") and are logged; its "More" button opens the same menu.
    /// The menu is anchored at a point, or to the button's element.
    #[component]
    fn MenuPage(anchored: Anchored) -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let mut open = use_signal(|| false);
        let mut button = use_signal(|| None::<MountedRef>);
        let anchor = match (anchored, button()) {
            (Anchored::ToButton, Some(element)) => Anchor::Mounted(element),
            _ => Anchor::Point(Point {
                x: Px(40.0),
                y: Px(80.0),
            }),
        };
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window,
                div { class: "app", tabindex: "0", style: "padding:20px; width:300px; height:200px",
                    onkeydown: move |event: KeyboardEvent| {
                        log.with_mut(|log| log.push(format!("key:{}", event.key())));
                        if event.key() == dioxus::prelude::Key::Character("m".to_owned()) {
                            open.set(true);
                        }
                    },
                    div { class: "bar", style: "display:flex; height:32px",
                        Button { common: Common { mounted: Some(EventHandler::new(move |event: MountedEvent| button.set(Some(MountedRef(event.data()))))), ..Common::default() }, label: "More",
                            onclick: move |_: Press| open.set(true),
                        }
                    }
                    p { class: "log", {log().join(",")} }
                }
                if open() {
                    Menu::<u8> {
                        placement: MenuPlacement::Bar,
                        anchor,
                        items: entries(),
                        onpick: |_| {},
                        onclose: move |()| open.set(false),
                    }
                }
            }
        }
    }

    /// What the page's menu is anchored to.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Anchored {
        /// A point: the menu knows no element to give the keyboard back to.
        AtPoint,
        /// The More button's element.
        ToButton,
    }

    fn at_point() -> Element {
        rsx! { MenuPage { anchored: Anchored::AtPoint } }
    }

    fn to_button() -> Element {
        rsx! { MenuPage { anchored: Anchored::ToButton } }
    }

    /// As `to_button`, but a pick removes the button (the pick swapped the view it stood in).
    #[component]
    fn SwappedPage() -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let mut open = use_signal(|| false);
        let mut swapped = use_signal(|| false);
        let mut button = use_signal(|| None::<MountedRef>);
        let anchor = button().map_or(
            Anchor::Point(Point {
                x: Px(40.0),
                y: Px(80.0),
            }),
            Anchor::Mounted,
        );
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window,
                div { class: "app", tabindex: "0", style: "padding:20px; width:300px; height:200px",
                    onkeydown: move |event: KeyboardEvent| {
                        log.with_mut(|log| log.push(format!("key:{}", event.key())));
                        if event.key() == dioxus::prelude::Key::Character("m".to_owned()) {
                            open.set(true);
                        }
                    },
                    div { class: "bar", style: "display:flex; height:32px",
                        if !swapped() {
                            Button { common: Common { mounted: Some(EventHandler::new(move |event: MountedEvent| button.set(Some(MountedRef(event.data()))))), ..Common::default() }, label: "More",
                                onclick: move |_: Press| open.set(true),
                            }
                        }
                    }
                    p { class: "log", {log().join(",")} }
                }
                if open() {
                    Menu::<u8> {
                        placement: MenuPlacement::Bar,
                        anchor,
                        items: entries(),
                        onpick: move |_| swapped.set(true),
                        onclose: move |()| open.set(false),
                    }
                }
            }
        }
    }

    /// A shell holding a pane (itself focusable) holding a field that removes itself on Enter, as
    /// a rename field does when it commits.
    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn FieldPage() -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let mut editing = use_signal(|| true);
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window,
                div { class: "app", tabindex: "0", style: "padding:20px; width:300px",
                    onkeydown: move |event: KeyboardEvent| log.with_mut(|log| log.push(format!("key:{}", event.key()))),
                    div { class: "pane", tabindex: "0", style: "padding:8px",
                        div { class: "wrap", style: "display:flex; height:32px",
                            if editing() {
                                TextField { label: "Name", value: "Inbox",
                                    oninput: |_| {},
                                    onkey: move |event: KeyboardEvent| {
                                        if event.key() == dioxus::prelude::Key::Named(dioxus::prelude::NamedKey::Enter) {
                                            editing.set(false);
                                        }
                                    },
                                }
                            }
                        }
                    }
                    p { class: "log", {log().join(",")} }
                }
            }
        }
    }

    fn harness(app: fn() -> Element, fallback: FocusFallback) -> Harness {
        let mut harness = Harness::new(
            app,
            HarnessConfig::new(VIEW)
                .with_clock(Clock::Virtual)
                .with_focus_fallback(fallback),
        );
        harness.advance(ms(50));
        harness
    }

    fn click(harness: &mut Harness, selector: &str) {
        let at = harness
            .centre(selector)
            .unwrap_or_else(|| panic!("{selector} is not laid out:\n{}", harness.html()));
        harness.send(Input::click(at));
    }

    fn log(harness: &Harness) -> String {
        harness.text_of(".log").unwrap_or_default()
    }

    /// Open the menu with `open`, wait for its panel to take the keyboard, close it with Escape and
    /// wait for it to leave.
    fn open_and_escape(harness: &mut Harness, open: impl FnOnce(&mut Harness)) {
        open(harness);
        settle_until(harness, |harness| harness.focus_of(".ds-menu"));
        harness.send(Input::key(ShortcutKey::Escape));
        settle_until(harness, |harness| harness.count(".ds-menu") == 0);
    }

    #[test]
    fn a_menu_opened_from_a_button_and_closed_with_escape_leaves_the_keyboard_in_the_app() {
        let mut harness = harness(at_point, FocusFallback::Ancestor);
        open_and_escape(&mut harness, |harness| click(harness, ".bar .ds-button"));
        settle_until(&mut harness, |harness| {
            harness.focus_of(".bar .ds-button") || harness.focus_of(".app")
        });
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(log(&harness), "key:j");
    }

    #[test]
    fn a_menu_whose_opener_the_pick_removes_hands_the_keyboard_to_an_ancestor() {
        let mut harness = harness(SwappedPage, FocusFallback::Ancestor);
        click(&mut harness, ".log");
        settle_until(&mut harness, |harness| harness.focus_of(".app"));
        harness.send(Input::key(ShortcutKey::Char('m')));
        settle_until(&mut harness, |harness| harness.focus_of(".ds-menu"));
        harness.send(Input::key(ShortcutKey::Down));
        harness.send(Input::key(ShortcutKey::Enter));
        settle_until(&mut harness, |harness| harness.count(".ds-menu") == 0);
        settle_until(&mut harness, |harness| harness.focus_of(".app"));
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(log(&harness), "key:m,key:j");
    }

    #[test]
    fn a_menu_anchored_to_an_element_gives_the_keyboard_back_to_it() {
        // Opened by a key on the app, so the button never had the keyboard: only the menu's own
        // hand-back to its anchor puts it there (the host's fallback would pick the app).
        let mut harness = harness(to_button, FocusFallback::Ancestor);
        click(&mut harness, ".log");
        settle_until(&mut harness, |harness| harness.focus_of(".app"));
        open_and_escape(&mut harness, |harness| {
            harness.send(Input::key(ShortcutKey::Char('m')));
        });
        settle_until(&mut harness, |harness| harness.focus_of(".bar .ds-button"));
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(log(&harness), "key:m,key:j");
    }

    #[test]
    fn with_blitz_default_the_focus_stays_nowhere_after_escape() {
        let mut harness = harness(at_point, FocusFallback::BlitzDefault);
        open_and_escape(&mut harness, |harness| click(harness, ".bar .ds-button"));
        harness.advance(ms(100));
        assert!(!harness.focus_of(".app"));
        assert!(!harness.focus_of(".bar .ds-button"));
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(log(&harness), "");
    }

    #[test]
    fn a_field_removed_by_its_own_handler_leaves_the_keyboard_on_its_nearest_focusable_ancestor() {
        let mut harness = harness(FieldPage, FocusFallback::Ancestor);
        click(&mut harness, ".wrap input");
        settle_until(&mut harness, |harness| harness.focus_of(".wrap input"));
        harness.send(Input::key(ShortcutKey::Enter));
        settle_until(&mut harness, |harness| harness.focus_of(".pane"));
        assert_eq!(harness.count(".wrap input"), 0, "the field left");
        harness.send(Input::key(ShortcutKey::Char('j')));
        assert_eq!(log(&harness), "key:Enter,key:j");
    }
}

mod field_handle {
    //! A field reached by its `FieldHandle` or by selector takes the keyboard,
    //! selects its value, and hears `onfocus` once (Blitz raises the event for a focus set from code,
    //! after the ask's own task has run, so a selector's "found" comes first);
    //! a handle's blur is heard once too (Blitz clears the focus silently, so the handle tells it); an unknown or unreadable selector is a typed error.

    use dioxus::prelude::*;
    use std::time::Duration;
    use ui_components::base::time::clock::sleep;
    use ui_components::focus::field::{FieldHandle, use_field_handle};
    use ui_components::focus::select::Select;
    use ui_components::focus::selector::{FocusError, focus_by_selector};
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 360,
        height: 320,
        scale_percent: 100,
    };

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// How the page's buttons reach the field.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Reach {
        /// Through the field's handle.
        Handle,
        /// Through `focus_by_selector(selector)`.
        Selector(&'static str),
    }

    /// Ask for the focus as `reach` says, logging a selector's outcome.
    fn ask(reach: Reach, handle: FieldHandle, mut log: Signal<Vec<String>>) {
        match reach {
            Reach::Handle => handle.focus(Select::All),
            Reach::Selector(selector) => {
                spawn(async move {
                    let outcome = match focus_by_selector(selector, Select::All).await {
                        Ok(()) => "found".to_owned(),
                        Err(FocusError::NoSuchElement { .. }) => "no-such-element".to_owned(),
                        Err(FocusError::BadSelector { .. }) => "bad-selector".to_owned(),
                        Err(other) => format!("{other:?}"),
                    };
                    log.with_mut(|log| log.push(outcome));
                });
            }
        }
    }

    /// A rename field on "Inbox" logging its focus and blur; "Go" reaches it as `reach` says, "Out"
    /// blurs it through its handle.
    #[component]
    fn Page(reach: Reach) -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let handle = use_field_handle();
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Sheet,
                div { style: "display:flex; flex-direction:column; gap:12px; width:300px; padding:12px",
                    div { class: "rename", style: "display:flex",
                        TextField { label: "Name", value: "Inbox",
                            handle: Some(handle),
                            oninput: |_| {},
                            onfocus: move |()| log.with_mut(|log| log.push("focus".to_owned())),
                            onblur: move |()| log.with_mut(|log| log.push("blur".to_owned())),
                        }
                    }
                    div { id: "go", style: "display:flex",
                        Button { label: "Go",
                            onclick: move |_| ask(reach, handle, log) }
                    }
                    div { id: "out", style: "display:flex",
                        Button { label: "Out",
                            onclick: move |_| handle.blur() }
                    }
                }
                p { class: "log", {log().join(",")} }
            }
        }
    }

    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn ByHandle() -> Element {
        rsx! { Page { reach: Reach::Handle } }
    }

    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn BySelector() -> Element {
        rsx! { Page { reach: Reach::Selector(".rename input") } }
    }

    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn ByUnknown() -> Element {
        rsx! { Page { reach: Reach::Selector("#nothing-here") } }
    }

    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn ByGarbage() -> Element {
        rsx! { Page { reach: Reach::Selector("[[") } }
    }

    fn log(harness: &Harness) -> String {
        harness.text_of(".log").unwrap_or_default()
    }

    fn press(harness: &mut Harness, selector: &str) {
        let at = harness
            .centre(selector)
            .unwrap_or_else(|| panic!("{selector} is not in the document:\n{}", harness.html()));
        harness.send(Input::click(at));
    }

    fn reached(app: fn() -> Element) -> Harness {
        let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        harness.advance(ms(50));
        press(&mut harness, "#go button");
        harness.advance(ms(150));
        harness
    }

    #[test]
    fn a_field_reached_by_handle_or_selector_hears_focus_once_with_its_value_selected() {
        // the page, and what its log says: a selector's "found" comes before the focus event
        type Case = (fn() -> Element, &'static str);
        let cases: [Case; 2] = [(ByHandle, "focus"), (BySelector, "found,focus")];
        for (app, logged) in cases {
            let harness = reached(app);
            assert!(harness.focus_of(".rename input"));
            assert_eq!(log(&harness), logged);
            assert_eq!(
                harness.selected_text(".rename input").as_deref(),
                Some("Inbox")
            );
        }
    }

    #[test]
    fn an_unknown_or_unreadable_selector_is_a_typed_error() {
        type Case = (fn() -> Element, &'static str);
        let cases: [Case; 2] = [(ByUnknown, "no-such-element"), (ByGarbage, "bad-selector")];
        for (app, logged) in cases {
            let mut harness =
                Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
            harness.advance(ms(50));
            press(&mut harness, "#go button");
            harness.advance(ms(1_000));
            assert_eq!(log(&harness), logged);
            assert!(!harness.focus_of(".rename input"));
        }
    }

    #[test]
    fn a_handle_blur_is_heard_once_and_leaves_the_field() {
        let mut harness = reached(ByHandle);
        press(&mut harness, "#out button");
        harness.advance(ms(150));
        assert!(!harness.focus_of(".rename input"));
        assert_eq!(log(&harness), "focus,blur");
    }

    /// A field asked for twice by selector, 300 ms apart, as a palette's does on each open; its text
    /// echoed below.
    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn Refocused() -> Element {
        let mut text = use_signal(|| "Invoice".to_owned());
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Sheet,
                div { class: "rename", style: "display:flex; width:300px; padding:12px",
                    TextField { label: "Name", value: text(), oninput: move |next| text.set(next) }
                }
                div { id: "go", style: "display:flex",
                    Button { label: "Go",
                        onclick: move |_| {
                            spawn(async move {
                                for _ in 0..2 {
                                    let _ = focus_by_selector(".rename input", Select::None).await;
                                    sleep(ms(300)).await;
                                }
                            });
                        }
                    }
                }
                p { class: "text", {text()} }
            }
        }
    }

    #[test]
    fn asking_again_for_a_focused_field_leaves_its_caret_where_it_is() {
        let mut harness = Harness::new(
            Refocused,
            HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
        );
        harness.advance(ms(50));
        press(&mut harness, "#go button");
        harness.advance(ms(150));
        harness.send(Input::key(ShortcutKey::Char('X')));
        harness.advance(ms(300));
        harness.send(Input::key(ShortcutKey::Char('Y')));
        harness.advance(ms(50));
        let text = harness.text_of(".text").unwrap_or_default();
        assert!(
            text.contains("XY"),
            "the second ask moved the caret: {text}"
        );
    }
}

mod click_restore {
    //! A click's focus restore skips an element the click removed. mailo's
    //! "Show images" button removes itself on press; the click's fallback had found that button (a
    //! `button` is focusable) and focused it a frame later, after it had left the document, so the
    //! keys that followed reached nothing. The restore now checks each remembered candidate when it
    //! focuses, and falls to the next focusable ancestor (`.app[tabindex]`).

    use dioxus::prelude::*;
    use std::time::Duration;
    use ui_components::base::press::Press;
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 240,
        scale_percent: 100,
    };

    /// Whether the banner's button removes itself when pressed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum OnPress {
        /// It goes, as "Show images" does once the images are shown.
        Leaves,
        /// It stays: the control.
        Stays,
    }

    #[component]
    fn Page(on_press: OnPress) -> Element {
        let mut log = use_signal(Vec::<String>::new);
        let mut shown = use_signal(|| true);
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window,
                div { class: "app", tabindex: "0", style: "padding:20px; width:300px",
                    onkeydown: move |event: KeyboardEvent| log.with_mut(|log| log.push(format!("key:{}", event.key()))),
                    div { class: "banner", style: "display:flex; height:40px",
                        if shown() {
                            Button { label: "Show images",
                                onclick: move |_: Press| {
                                    log.with_mut(|log| log.push("show".to_owned()));
                                    if on_press == OnPress::Leaves {
                                        shown.set(false);
                                    }
                                },
                            }
                        }
                    }
                    p { class: "log", {log().join(",")} }
                }
            }
        }
    }

    fn leaves() -> Element {
        rsx! { Page { on_press: OnPress::Leaves } }
    }

    fn stays() -> Element {
        rsx! { Page { on_press: OnPress::Stays } }
    }

    fn harness(on_press: OnPress) -> Harness {
        let app = match on_press {
            OnPress::Leaves => leaves,
            OnPress::Stays => stays,
        };
        let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        harness.advance(Duration::from_millis(50));
        harness
    }

    fn press_show(harness: &mut Harness) {
        let at = harness
            .centre(".banner .ds-button")
            .unwrap_or_else(|| panic!("the button is not laid out:\n{}", harness.html()));
        harness.send(Input::click(at));
    }

    #[test]
    fn a_button_that_removes_itself_leaves_the_keyboard_on_its_focusable_ancestor() {
        // What the press does, where the keyboard ends, and how many buttons are left.
        let cases = [
            (OnPress::Leaves, ".app", 0),
            (OnPress::Stays, ".banner .ds-button", 1),
        ];
        for (on_press, focused, left) in cases {
            let mut harness = harness(on_press);
            press_show(&mut harness);
            ui_harness::harness::settle_until(&mut harness, |harness| harness.focus_of(focused));
            assert_eq!(harness.count(".banner .ds-button"), left);
            harness.send(Input::key(ShortcutKey::Char('j')));
            assert_eq!(harness.text_of(".log").as_deref(), Some("show,key:j"));
        }
    }
}

mod select {
    //! A controlled focus that selects the field's whole value once the caret
    //! lands (a rename field opened on the old name), and the public `focus_soon` an app uses for
    //! its own element.

    use dioxus::prelude::*;
    use std::rc::Rc;
    use std::time::Duration;
    use ui_components::focus::request::use_focus_request;
    use ui_components::focus::soon::focus_soon;
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 360,
        height: 240,
        scale_percent: 100,
    };

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// A rename field on "Archive", focused with select-all on mount and by the button; a second
    /// field to move away to; the value echoed below.
    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn Rename() -> Element {
        let mut name = use_signal(|| "Archive".to_owned());
        let request = use_focus_request().with_select_all();
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Sheet,
                div { style: "display:flex; flex-direction:column; gap:12px; width:300px; padding:12px",
                    div { id: "name", style: "display:flex",
                        TextField { label: "Name", value: name(),
                            focus: FieldFocus::Controlled(request), oninput: move |value| name.set(value) }
                    }
                    div { id: "other", style: "display:flex",
                        TextField { label: "Other", value: "", oninput: |_| {} }
                    }
                    div { id: "again", style: "display:flex",
                        Button { label: "Rename",
                            onclick: move |_| request.request() }
                    }
                }
                p { class: "name", {name()} }
            }
        }
    }

    /// The same field, controlled but without select-all.
    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn Plain() -> Element {
        let request = use_focus_request();
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Sheet,
                div { id: "name", style: "display:flex; width:300px; padding:12px",
                    TextField { label: "Name", value: "Archive",
                        focus: FieldFocus::Controlled(request), oninput: |_| {} }
                }
            }
        }
    }

    #[test]
    fn a_controlled_focus_with_select_all_selects_the_whole_value() {
        let mut harness = Harness::new(Rename, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        harness.advance(ms(100));
        assert!(harness.focus_of("#name input"));
        assert_eq!(
            harness.selected_text("#name input").as_deref(),
            Some("Archive")
        );
        harness.send(Input::key(ShortcutKey::Char('X')));
        harness.advance(ms(20));
        assert_eq!(
            harness.text_of(".name").as_deref(),
            Some("X"),
            "typing replaced it"
        );
    }

    /// A laid-out field asked for again is focused and selected in one write: no frame sees it
    /// focused with its caret not yet placed, so a key typed then is never moved away from.
    #[test]
    fn the_value_is_selected_in_the_frame_the_focus_lands() {
        let mut harness = Harness::new(Rename, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        harness.advance(ms(100));
        let other = harness.centre("#other input").expect("the other field");
        harness.send(Input::click(other));
        harness.advance(ms(50));
        let again = harness.centre("#again .ds-button").expect("the button");
        harness.send(Input::click(again));
        let mut frames = 0;
        while !harness.focus_of("#name input") {
            if !harness.step_frame() {
                harness.advance(ms(1));
            }
            frames += 1;
            assert!(frames < 60, "never focused");
        }
        assert_eq!(
            harness.selected_text("#name input").as_deref(),
            Some("Archive"),
            "the caret was placed in a later write than the focus"
        );
    }

    #[test]
    fn a_controlled_focus_without_select_all_selects_nothing() {
        let mut harness = Harness::new(Plain, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        harness.advance(ms(100));
        assert!(harness.focus_of("#name input"));
        assert_eq!(harness.selected_text("#name input"), None);
    }

    /// An app's own focusable element, focused through the public seam from a button.
    #[expect(
        non_snake_case,
        reason = "a Dioxus component function is named in CamelCase"
    )]
    fn Shell() -> Element {
        let mut shell = use_signal(|| None::<Rc<MountedData>>);
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Sheet,
                div { class: "app", tabindex: "0", onmounted: move |event| shell.set(Some(event.data())),
                    div { id: "back", style: "display:flex; width:200px",
                        Button { label: "Back",
                            onclick: move |_| if let Some(element) = shell() { focus_soon(element) } }
                    }
                }
            }
        }
    }

    #[test]
    fn an_app_focuses_its_own_element_through_focus_soon() {
        let mut harness = Harness::new(Shell, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        assert!(!harness.focus_of(".app"));
        let back = harness.centre("#back .ds-button").expect("the button");
        harness.send(Input::click(back));
        harness.advance(ms(50));
        assert!(harness.focus_of(".app"), "{}", harness.html());
    }
}

//! Iframes and the host phases: a `srcdoc` frame as its own document, the links, link text and
//! hover, context menu and image clicks the app hears from inside one, the tag each frame carries,
//! and the frame phase (watched rects and queued scrolls). One module per scenario, each with its
//! own page.

mod document {
    //! An `<iframe srcdoc>` is a real, separate sub-document under the harness,
    //! as in the window: its text renders, its selectors are found through the frame and never
    //! through the app's document, and a `<script>` in it does nothing (Blitz runs no scripts).

    use dioxus::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Viewport};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 300,
        scale_percent: 100,
    };

    /// A sender-shaped body: its own style, text, and a script that would rewrite it.
    const BODY: &str = "<html><head><style>p { color: red }</style></head><body>\
    <p class='inner'>Hello from the frame</p>\
    <script>document.body.innerHTML = '<p class=\"pwned\">rewritten</p>';</script>\
    </body></html>";

    #[allow(non_snake_case)]
    fn Reader() -> Element {
        rsx! {
            p { class: "outer", "The app's own text" }
            iframe { class: "body", srcdoc: BODY, style: "width: 300px; height: 150px" }
        }
    }

    #[test]
    fn a_srcdoc_frame_is_its_own_inert_document() {
        let harness = Harness::new(Reader, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        let frame = harness
            .frame("iframe.body")
            .unwrap_or_else(|| panic!("no frame document:\n{}", harness.html()));
        assert_eq!(
            frame.text_of(".inner").as_deref(),
            Some("Hello from the frame")
        );
        assert_eq!(frame.count("p.inner"), 1);
        assert!(frame.text().contains("Hello from the frame"));
        // Separate documents: neither sees the other's nodes, and the frame's script ran nothing.
        assert_eq!(harness.count(".inner"), 0, "the app saw into the frame");
        assert_eq!(frame.count(".outer"), 0, "the frame saw the app");
        assert_eq!(frame.count(".pwned"), 0);
    }

    #[test]
    fn the_frames_text_is_painted_in_its_own_colour() {
        let mut harness = Harness::new(Reader, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        let frame = harness.render().expect("renders");
        let at = harness.rect("iframe.body").expect("the frame's box");
        let (x0, y0) = (at.origin.x.0 as u32, at.origin.y.0 as u32);
        let (x1, y1) = (x0 + at.size.width.0 as u32, y0 + at.size.height.0 as u32);
        let red = (x0..x1.min(frame.width()))
            .flat_map(|x| (y0..y1.min(frame.height())).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let [r, g, b, _] = frame.get_pixel(x, y).0;
                r > 150 && g < 90 && b < 90
            })
            .count();
        assert!(
            red > 10,
            "no red text painted inside the frame ({red} pixels)"
        );
    }
}

mod links {
    //! A link clicked inside a frame never navigates the frame. With
    //! `FrameLinks::intercept` the app hears the click (mailo opens it in the browser); with the
    //! default (inert) nothing happens. Either way no request is made for the link's target.

    use dioxus::prelude::*;
    use ui_blitz::{AppNet, FrameLink, FrameLinks, NetDecision, NetPolicy, NetReply, NetRequest};
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use std::sync::{Arc, Mutex, PoisonError};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 300,
        scale_percent: 100,
    };

    const BODY: &str = "<body style='margin:0'>\
    <a class='link' href='https://example.org/offer' style='display:block; font-size:40px'>Offer</a>\
    </body>";

    #[allow(non_snake_case)]
    fn Reader() -> Element {
        rsx! {
            iframe { class: "body", srcdoc: BODY, style: "width: 300px; height: 150px; border: 0" }
        }
    }

    /// Every request any document makes, refused.
    #[derive(Default)]
    struct Requests(Mutex<Vec<String>>);

    impl AppNet for Requests {
        fn decide(&self, request: &NetRequest) -> NetDecision {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(request.url().to_owned());
            NetDecision::Deny
        }

        fn fetch(&self, _request: NetRequest, _reply: NetReply) {}
    }

    impl Requests {
        fn seen(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    /// Build the reader with `links`, click the frame's link, and say what the frame was before and
    /// after, and what was requested.
    fn click_the_link(links: FrameLinks) -> (Harness, ui_blitz::FrameId, Arc<Requests>) {
        let requests = Arc::new(Requests::default());
        let config = HarnessConfig::new(VIEW)
            .with_clock(Clock::Virtual)
            .with_net(NetPolicy::Custom(requests.clone()))
            .with_frame_links(links);
        let mut harness = Harness::new(Reader, config);
        let frame = harness.frame("iframe.body").expect("a frame document");
        let before = frame.id();
        let at = frame.centre("a.link").expect("the link in the frame");
        harness.send(Input::click(at));
        harness.advance(std::time::Duration::from_millis(50));
        (harness, before, requests)
    }

    #[test]
    fn an_intercepted_click_reaches_the_app_and_the_frame_stays() {
        let heard = Arc::new(Mutex::new(Vec::<FrameLink>::new()));
        let log = Arc::clone(&heard);
        let links = FrameLinks::intercept(move |link| {
            log.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(link);
        });
        let (harness, before, requests) = click_the_link(links);
        let heard = heard.lock().unwrap_or_else(PoisonError::into_inner).clone();
        assert_eq!(
            heard,
            vec![FrameLink {
                frame: before,
                tag: None,
                href: "https://example.org/offer".to_owned(),
                text: "Offer".to_owned(),
                title: None,
            }]
        );
        let after = harness
            .frame("iframe.body")
            .expect("still a frame document");
        assert_eq!(after.id(), before, "the frame was reloaded");
        assert_eq!(after.text_of("a.link").as_deref(), Some("Offer"));
        assert!(
            requests.seen().is_empty(),
            "requested {:?}",
            requests.seen()
        );
    }

    #[test]
    fn an_inert_click_does_nothing() {
        let (harness, before, requests) = click_the_link(FrameLinks::default());
        let after = harness
            .frame("iframe.body")
            .expect("still a frame document");
        assert_eq!(after.id(), before, "the frame was reloaded");
        assert!(
            requests.seen().is_empty(),
            "requested {:?}",
            requests.seen()
        );
    }
}

mod link_text {
    //! What a link in a frame says, for mailo's link honesty check and its link pill: a click
    //! carries the anchor's visible text (whitespace-collapsed) and title beside its destination,
    //! and with `FrameLinks::with_hover` the app hears the pointer come onto and leave each link,
    //! once per crossing, at the pointer's point in the app's document.

    use dioxus::prelude::*;
    use ui_components::prelude::*;
    use ui_blitz::{FrameLink, FrameLinkHover, FrameLinks, FrameTag, HoverPhase};
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::Duration;

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    const BODY: &str = "<body style='margin:0; font-size:30px'>\
    <a class='bank' href='https://bank.example.net/login' title='Sign in' style='display:block'>\
      Your\n   bank </a>\
    <p class='plain' style='margin:0'>Nothing here</p>\
    <a class='offer' href='https://example.org/offer' style='display:block'>Offer</a>\
    </body>";

    #[allow(non_snake_case)]
    fn Reader() -> Element {
        rsx! {
            div { class: "above", style: "height: 40px", "Subject" }
            iframe {
                class: "body",
                "data-frame-tag": "msg-7",
                srcdoc: BODY,
                style: "display: block; margin-left: 20px; width: 300px; height: 200px; border: 0",
            }
        }
    }

    /// Everything the app heard.
    #[derive(Default)]
    struct Heard {
        clicks: Mutex<Vec<FrameLink>>,
        hovers: Mutex<Vec<FrameLinkHover>>,
    }

    impl Heard {
        fn clicks(&self) -> Vec<FrameLink> {
            self.clicks
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        fn hovers(&self) -> Vec<FrameLinkHover> {
            self.hovers
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    /// The reader, with the app listening to clicks and hovers.
    fn reader() -> (Harness, Arc<Heard>) {
        let heard = Arc::new(Heard::default());
        let (clicks, hovers) = (Arc::clone(&heard), Arc::clone(&heard));
        let links = FrameLinks::intercept(move |link| {
            clicks
                .clicks
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(link);
        })
        .with_hover(move |hover| {
            hovers
                .hovers
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(hover);
        });
        let config = HarnessConfig::new(VIEW)
            .with_clock(Clock::Virtual)
            .with_frame_links(links);
        (Harness::new(Reader, config), heard)
    }

    /// The centre of `selector` in the frame, in the app document's coordinates.
    fn at(harness: &Harness, selector: &str) -> Point {
        harness
            .frame("iframe.body")
            .and_then(|frame| frame.centre(selector))
            .unwrap_or_else(|| panic!("{selector} in the frame"))
    }

    /// `point` moved right by `dx`.
    fn nudged(point: Point, dx: f32) -> Point {
        Point {
            x: Px(point.x.0 + dx),
            y: point.y,
        }
    }

    #[test]
    fn a_click_carries_the_links_text_and_title() {
        let (mut harness, heard) = reader();
        let bank = at(&harness, "a.bank");
        harness.send(Input::click(bank));
        let offer = at(&harness, "a.offer");
        harness.send(Input::click(offer));
        harness.advance(Duration::from_millis(50));
        let said: Vec<_> = heard
            .clicks()
            .into_iter()
            .map(|link| (link.href, link.text, link.title))
            .collect();
        assert_eq!(
            said,
            vec![
                (
                    "https://bank.example.net/login".to_owned(),
                    "Your bank".to_owned(),
                    Some("Sign in".to_owned())
                ),
                (
                    "https://example.org/offer".to_owned(),
                    "Offer".to_owned(),
                    None
                ),
            ]
        );
    }

    #[test]
    fn moving_onto_a_link_reports_enter_once_and_moving_off_reports_leave() {
        let (mut harness, heard) = reader();
        let bank = at(&harness, "a.bank");
        let frame = harness.frame("iframe.body").expect("the frame").id();
        harness.send(Input::pointer_move(bank));
        harness.send(Input::pointer_move(nudged(bank, 5.0)));
        harness.send(Input::pointer_move(nudged(bank, -5.0)));
        let plain = at(&harness, "p.plain");
        harness.send(Input::pointer_move(plain));
        let hovers = heard.hovers();
        let enter = FrameLinkHover {
            frame,
            tag: Some(FrameTag::new("msg-7")),
            href: "https://bank.example.net/login".to_owned(),
            text: "Your bank".to_owned(),
            title: Some("Sign in".to_owned()),
            at: bank,
            phase: HoverPhase::Enter,
        };
        let leave = FrameLinkHover {
            at: plain,
            phase: HoverPhase::Leave,
            ..enter.clone()
        };
        assert_eq!(hovers, vec![enter, leave]);
    }

    #[test]
    fn moving_from_one_link_to_another_leaves_then_enters() {
        let (mut harness, heard) = reader();
        let bank = at(&harness, "a.bank");
        let offer = at(&harness, "a.offer");
        harness.send(Input::pointer_move(bank));
        harness.send(Input::pointer_move(offer));
        let above = harness.centre("div.above").expect("the app's own text");
        harness.send(Input::pointer_move(above));
        let crossings: Vec<_> = heard
            .hovers()
            .into_iter()
            .map(|hover| (hover.phase, hover.text, hover.at))
            .collect();
        assert_eq!(
            crossings,
            vec![
                (HoverPhase::Enter, "Your bank".to_owned(), bank),
                (HoverPhase::Leave, "Your bank".to_owned(), offer),
                (HoverPhase::Enter, "Offer".to_owned(), offer),
                (HoverPhase::Leave, "Offer".to_owned(), above),
            ]
        );
    }

    #[test]
    fn inert_links_report_no_hover() {
        let heard = Arc::new(Heard::default());
        let log = Arc::clone(&heard);
        let links = FrameLinks::default().with_hover(move |hover| {
            log.hovers
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(hover);
        });
        let mut harness = Harness::new(
            Reader,
            HarnessConfig::new(VIEW)
                .with_clock(Clock::Virtual)
                .with_frame_links(links),
        );
        let bank = at(&harness, "a.bank");
        harness.send(Input::pointer_move(bank));
        let plain = at(&harness, "p.plain");
        harness.send(Input::pointer_move(plain));
        assert_eq!(heard.hovers(), Vec::new());
    }
}

mod link_menu {
    //! The link under a context menu inside a frame, for mailo's "Copy Link": with
    //! `FrameLinks::with_context_menu` the app hears a secondary press released over a link in a
    //! frame, with the link's target resolved against the frame's base URL, its text, its title and the
    //! pointer's point; nothing for a press off every link, for a primary click, or when the
    //! frames' links are inert. The press still reaches the page's own `oncontextmenu`.

    use dioxus::prelude::*;
    use ui_components::base::press::PointerButton;
    use ui_components::prelude::*;
    use ui_blitz::{FrameLink, FrameLinkMenu, FrameLinks, FrameTag};
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::Duration;

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    const BODY: &str = "<body style='margin:0; font-size:30px'>\
    <a class='bank' href='https://bank.example.net/login?next=%2F' title='Sign in' style='display:block'>\
      <span class='inner'>Your\n   bank</span> </a>\
    <p class='plain' style='margin:0'>Nothing here</p>\
    </body>";

    #[allow(non_snake_case)]
    fn Reader() -> Element {
        let mut asked = use_signal(|| 0u32);
        rsx! {
            div {
                class: "zone",
                oncontextmenu: move |_| asked += 1,
                iframe {
                    class: "body",
                    "data-frame-tag": "msg-7",
                    srcdoc: BODY,
                    style: "display: block; margin-left: 20px; width: 300px; height: 200px; border: 0",
                }
            }
            for _ in 0..asked() {
                i { class: "asked" }
            }
        }
    }

    #[derive(Default)]
    struct Heard {
        menus: Mutex<Vec<FrameLinkMenu>>,
    }

    impl Heard {
        fn menus(&self) -> Vec<FrameLinkMenu> {
            self.menus
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    fn reader(links: FrameLinks) -> (Harness, Arc<Heard>) {
        let heard = Arc::new(Heard::default());
        let log = Arc::clone(&heard);
        let links = links.with_context_menu(move |menu| {
            log.menus
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(menu);
        });
        let config = HarnessConfig::new(VIEW)
            .with_clock(Clock::Virtual)
            .with_frame_links(links);
        (Harness::new(Reader, config), heard)
    }

    fn listening() -> (Harness, Arc<Heard>) {
        reader(FrameLinks::intercept(|_: FrameLink| {}))
    }

    fn at(harness: &Harness, selector: &str) -> Point {
        harness
            .frame("iframe.body")
            .and_then(|frame| frame.centre(selector))
            .unwrap_or_else(|| panic!("{selector} in the frame"))
    }

    fn right_click(harness: &mut Harness, at: Point) {
        harness.send(Input::pointer_move(at));
        harness.send(Input::button_down(at, PointerButton::Secondary));
        harness.send(Input::button_up(at, PointerButton::Secondary));
        harness.advance(Duration::from_millis(50));
    }

    #[test]
    fn a_right_click_on_a_link_in_a_frame_reports_the_link_under_it() {
        let (mut harness, heard) = listening();
        // The word sits inside the anchor: the nearest enclosing `a[href]` is the link
        let word = at(&harness, "span.inner");
        let frame = harness.frame("iframe.body").expect("the frame").id();
        right_click(&mut harness, word);
        let want = FrameLinkMenu {
            frame,
            tag: Some(FrameTag::new("msg-7")),
            href: "https://bank.example.net/login?next=%2F".to_owned(),
            text: "Your bank".to_owned(),
            title: Some("Sign in".to_owned()),
            at: word,
        };
        assert_eq!(heard.menus(), vec![want]);
        assert_eq!(
            harness.count(".asked"),
            1,
            "the page's own menu still asked"
        );
    }

    #[test]
    fn a_right_click_off_every_link_or_a_left_click_on_one_reports_no_menu() {
        let (mut harness, heard) = listening();
        let plain = at(&harness, "p.plain");
        right_click(&mut harness, plain);
        assert_eq!(harness.count(".asked"), 1);
        let bank = at(&harness, "a.bank");
        harness.send(Input::click(bank));
        assert_eq!(heard.menus(), Vec::new());
    }
}

mod image_click {
    //! A click on an image inside a frame, for mailo's Peek of an inline image: with
    //! `FrameLinks::with_image_click` the app hears a primary release on an `img` in a frame, with its
    //! `src` (a `cid:` reference as written), its `alt`, the frame's tag and the pointer's point;
    //! nothing for an image inside a link (the link's click), for a press off every image, for a
    //! secondary press, or when the frames' links are inert.

    use dioxus::prelude::*;
    use ui_components::base::press::PointerButton;
    use ui_components::prelude::*;
    use ui_blitz::{FrameImageClick, FrameLink, FrameLinks, FrameTag};
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::Duration;

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    const BODY: &str = "<body style='margin:0'>\
    <img class='logo' src='cid:logo@example.org' alt='Logo' style='display:block; width:120px; height:60px'>\
    <a href='https://shop.example.net/' style='display:block'>\
      <img class='linked' src='cid:banner@example.org' style='display:block; width:120px; height:60px'></a>\
    <p class='plain' style='margin:0; font-size:30px'>Nothing here</p>\
    </body>";

    #[allow(non_snake_case)]
    fn Reader() -> Element {
        rsx! {
            iframe {
                class: "body",
                "data-frame-tag": "msg-7",
                srcdoc: BODY,
                style: "display: block; margin-left: 20px; width: 300px; height: 300px; border: 0",
            }
        }
    }

    #[derive(Default)]
    struct Heard {
        images: Mutex<Vec<FrameImageClick>>,
        links: Mutex<Vec<String>>,
    }

    impl Heard {
        fn images(&self) -> Vec<FrameImageClick> {
            self.images
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        fn links(&self) -> Vec<String> {
            self.links
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    fn reader(intercept: bool) -> (Harness, Arc<Heard>) {
        let heard = Arc::new(Heard::default());
        let (images, links) = (Arc::clone(&heard), Arc::clone(&heard));
        let base = if intercept {
            FrameLinks::intercept(move |link: FrameLink| {
                links
                    .links
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(link.href);
            })
        } else {
            FrameLinks::default()
        };
        let frame_links = base.with_image_click(move |click| {
            images
                .images
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(click);
        });
        let config = HarnessConfig::new(VIEW)
            .with_clock(Clock::Virtual)
            .with_frame_links(frame_links);
        (Harness::new(Reader, config), heard)
    }

    fn at(harness: &Harness, selector: &str) -> Point {
        harness
            .frame("iframe.body")
            .and_then(|frame| frame.centre(selector))
            .unwrap_or_else(|| panic!("{selector} in the frame"))
    }

    fn click(harness: &mut Harness, at: Point, button: PointerButton) {
        harness.send(Input::pointer_move(at));
        harness.send(Input::button_down(at, button));
        harness.send(Input::button_up(at, button));
        harness.advance(Duration::from_millis(50));
    }

    #[test]
    fn a_click_on_an_image_in_a_frame_reports_its_source_and_the_frame() {
        let (mut harness, heard) = reader(true);
        let logo = at(&harness, "img.logo");
        let frame = harness.frame("iframe.body").expect("the frame").id();
        click(&mut harness, logo, PointerButton::Primary);
        let want = FrameImageClick {
            frame,
            tag: Some(FrameTag::new("msg-7")),
            src: "cid:logo@example.org".to_owned(),
            alt: Some("Logo".to_owned()),
            at: logo,
        };
        assert_eq!(heard.images(), vec![want]);
    }

    #[test]
    fn an_image_inside_a_link_is_the_links_click() {
        let (mut harness, heard) = reader(true);
        let linked = at(&harness, "img.linked");
        click(&mut harness, linked, PointerButton::Primary);
        assert_eq!(heard.images(), Vec::new());
        assert_eq!(heard.links(), vec!["https://shop.example.net/".to_owned()]);
    }

    #[test]
    fn a_click_off_every_image_a_secondary_press_or_an_inert_frame_reports_nothing() {
        let (mut harness, heard) = reader(true);
        let plain = at(&harness, "p.plain");
        click(&mut harness, plain, PointerButton::Primary);
        let logo = at(&harness, "img.logo");
        click(&mut harness, logo, PointerButton::Secondary);
        assert_eq!(heard.images(), Vec::new());
        // And frames whose links are inert report no image either.
        let (mut inert, heard) = reader(false);
        let logo = at(&inert, "img.logo");
        click(&mut inert, logo, PointerButton::Primary);
        assert_eq!(heard.images(), Vec::new());
    }
}

mod tags {
    //! Frame tags: the app names each `iframe` with `data-frame-tag`, and everything its document
    //! does (a request put to the app; link, menu and image events carry it too) carries that name, so mailo never infers which
    //! message a frame shows from the URLs it asks for. `ui_blitz::frames` looks a frame up either
    //! way, from the id to the tag and back.

    use dioxus::prelude::*;
    use ui_blitz::{AppNet, FrameId, FrameTag, NetDecision, NetPolicy, NetReply, NetRequest};
    use ui_harness::{Clock, Harness, HarnessConfig, Viewport};
    use std::sync::{Arc, Mutex, PoisonError};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    const FIRST: &str = "<body style='margin:0'>\
    <img src='https://example.org/first.png'>\
    <a class='link' href='https://example.org/one' style='display:block; font-size:40px'>One</a>\
    </body>";

    const SECOND: &str = "<body style='margin:0'>\
    <img src='https://example.org/second.png'>\
    <a class='link' href='https://example.org/two' style='display:block; font-size:40px'>Two</a>\
    </body>";

    #[allow(non_snake_case)]
    fn Reader() -> Element {
        rsx! {
            iframe {
                class: "first",
                "data-frame-tag": "msg-1",
                srcdoc: FIRST,
                style: "display: block; width: 300px; height: 150px; border: 0",
            }
            iframe {
                class: "second",
                "data-frame-tag": "msg-2",
                srcdoc: SECOND,
                style: "display: block; width: 300px; height: 150px; border: 0",
            }
        }
    }

    /// One request as the app saw it.
    type Seen = (Option<FrameId>, Option<FrameTag>, String);

    /// Every request put to it, refused.
    #[derive(Default)]
    struct Recorder(Mutex<Vec<Seen>>);

    impl AppNet for Recorder {
        fn decide(&self, request: &NetRequest) -> NetDecision {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push((
                request.origin(),
                request.frame_tag().cloned(),
                request.url().to_owned(),
            ));
            NetDecision::Deny
        }

        fn fetch(&self, _request: NetRequest, _reply: NetReply) {}
    }

    /// The reader with `requests` as its network.
    fn reader(requests: Arc<Recorder>) -> Harness {
        let config = HarnessConfig::new(VIEW)
            .with_clock(Clock::Virtual)
            .with_net(NetPolicy::Custom(requests));
        Harness::new(Reader, config)
    }

    fn tag(text: &str) -> Option<FrameTag> {
        Some(FrameTag::new(text))
    }

    #[test]
    fn each_frames_request_carries_its_tag() {
        let requests = Arc::new(Recorder::default());
        let harness = reader(Arc::clone(&requests));
        let first = harness.frame("iframe.first").expect("the first frame").id();
        let second = harness
            .frame("iframe.second")
            .expect("the second frame")
            .id();
        let mut seen = requests
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        seen.sort_by(|a, b| a.2.cmp(&b.2));
        assert_eq!(
            seen,
            vec![
                (
                    Some(first),
                    tag("msg-1"),
                    "https://example.org/first.png".to_owned()
                ),
                (
                    Some(second),
                    tag("msg-2"),
                    "https://example.org/second.png".to_owned()
                ),
            ]
        );
    }
}

mod phase {
    //! The frame phase: a component watches an element's rect and queues a scroll through the host's
    //! geometry part, and the window loop's one step per frame (the harness runs it the same way)
    //! publishes the rect when it changes and applies the scroll, with the document free. Nothing
    //! here waits a frame or retries: the harness settles and the values are there.

    use dioxus::prelude::*;
    use ui_components::base::geometry::scroll::Scroll;
    use ui_components::host::document::{DocumentHost, use_document_host};
    use ui_components::host::measure::MountedRef;
    use ui_components::host::phase::{Observe, Observed, PhaseWrite, Watch};
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
    use std::rc::Rc;
    use std::time::Duration;

    const VIEW: Viewport = Viewport {
        width: 480,
        height: 320,
        scale_percent: 100,
    };

    /// A box that grows when its button is pressed, watched through the host.
    #[allow(non_snake_case)]
    fn Growing() -> Element {
        let host = use_document_host();
        let mut wide = use_signal(|| false);
        let rect = use_signal(|| None::<Rect>);
        let mut watch = use_signal(|| None::<Watch>);
        // Counted outside the signals, so counting does not itself change anything.
        let mut publishes = use_hook(|| CopyValue::new(0u32));
        use_effect(move || {
            let _ = rect();
            publishes += 1;
        });
        let width = if wide() { 200 } else { 100 };
        let shown = rect().map_or_else(
            || "none".to_owned(),
            |rect| format!("{}x{}", rect.size.width.0, rect.size.height.0),
        );
        rsx! {
            div { class: "box", style: "width:{width}px; height:40px",
                onmounted: move |mounted: MountedEvent| {
                    let observed = host.geometry().observe(&mounted.data(), Observe::Rect(rect));
                    if let Observed::Watching(found) = observed {
                        watch.set(Some(found));
                    }
                },
            }
            button { class: "grow", onclick: move |_| wide.set(true), "grow" }
            p { class: "rect", {shown} }
            p { class: "publishes", "{publishes}" }
        }
    }

    fn click(harness: &mut Harness, selector: &str) {
        let at = harness
            .centre(selector)
            .unwrap_or_else(|| panic!("{selector} is not laid out:\n{}", harness.html()));
        harness.send(Input::click(at));
    }

    #[test]
    fn a_watched_rect_is_published_after_layout_and_again_only_when_it_changes() {
        let mut harness =
            Harness::new(Growing, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        assert_eq!(harness.text_of(".rect").as_deref(), Some("100x40"));
        let first = harness.text_of(".publishes");

        click(&mut harness, ".grow");
        assert_eq!(harness.text_of(".rect").as_deref(), Some("200x40"));
        // Every frame since the box last moved found the same rect and published nothing.
        harness.advance(Duration::from_millis(200));
        let counts = (first, harness.text_of(".publishes"));
        assert_eq!(
            counts,
            (Some("1".to_owned()), Some("2".to_owned())),
            "one effect run for the rect first published, one for the width change"
        );
    }

    /// Queue `writes` to the scroller through the host.
    fn queue(host: &Rc<dyn DocumentHost>, scroller: Option<MountedRef>, writes: &[f32]) {
        let Some(MountedRef(element)) = scroller else {
            return;
        };
        for &to in writes {
            let queued = host
                .geometry()
                .write(&element, PhaseWrite::ScrollTo(Px(to)));
            assert!(queued);
        }
    }

    /// A scroller of 1000 px of content in 100, and buttons that queue writes to it.
    #[allow(non_snake_case)]
    fn Scrolling() -> Element {
        let host = use_document_host();
        let mut scroller = use_signal(|| None::<MountedRef>);
        let (one, many, past) = (host.clone(), host.clone(), host);
        rsx! {
            div { class: "scroller", style: "height:100px; overflow-y:auto; scrollbar-width:none",
                onmounted: move |mounted: MountedEvent| scroller.set(Some(MountedRef(mounted.data()))),
                div { style: "height:1000px" }
            }
            button { class: "to-300", onclick: move |_| queue(&one, scroller(), &[300.0]), "to 300" }
            button { class: "to-many", onclick: move |_| queue(&many, scroller(), &[40.0, 120.0, 250.0]), "to 250" }
            button { class: "past-end", onclick: move |_| queue(&past, scroller(), &[5000.0]), "past the end" }
        }
    }

    fn offset(harness: &Harness) -> f64 {
        harness
            .with_doc(|doc| {
                let scroller = doc.query_selector(".scroller").ok().flatten()?;
                Some(doc.get_node(scroller)?.scroll_offset().y)
            })
            .expect("the scroller is in the document")
    }

    #[test]
    fn a_queued_scroll_is_applied_in_the_frame_phase() {
        // (button, the offset it leaves the scroller at)
        const CASES: &[(&str, f64)] = &[
            (".to-300", 300.0),
            // Three writes in one handler are one scroll to the last.
            (".to-many", 250.0),
            // The scrollable range ends at 1000 - 100.
            (".past-end", 900.0),
        ];
        for &(button, want) in CASES {
            let mut harness = Harness::new(
                Scrolling,
                HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
            );
            assert_eq!(offset(&harness), 0.0, "{button}: starts at the top");
            click(&mut harness, button);
            assert_eq!(offset(&harness), want, "{button}");
        }
    }

    /// A scroller whose scroll state is watched through the host, and a button that scrolls it by a
    /// queued write: the document dispatches no `scroll` event for that, so only the phase sees it.
    #[allow(non_snake_case)]
    fn Watched() -> Element {
        let host = use_document_host();
        let mut scroller = use_signal(|| None::<MountedRef>);
        let seen = use_signal(|| None::<Scroll>);
        let mut watch = use_signal(|| None::<Watch>);
        let (watching, writing) = (host.clone(), host);
        let shown = seen().map_or_else(
            || "none".to_owned(),
            |seen| format!("{} {} {}", seen.offset.0, seen.viewport.0, seen.content.0),
        );
        rsx! {
            div { class: "scroller", style: "height:100px; overflow-y:auto; scrollbar-width:none",
                onmounted: move |mounted: MountedEvent| {
                    let observed = watching.geometry().observe(&mounted.data(), Observe::Scroll(seen));
                    if let Observed::Watching(found) = observed {
                        watch.set(Some(found));
                    }
                    scroller.set(Some(MountedRef(mounted.data())));
                },
                div { style: "height:1000px" }
            }
            button { class: "to-300", onclick: move |_| queue(&writing, scroller(), &[300.0]), "to 300" }
            p { class: "seen", {shown} }
        }
    }

    #[test]
    fn a_scroll_the_document_made_without_an_event_is_published() {
        let mut harness =
            Harness::new(Watched, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        assert_eq!(harness.text_of(".seen").as_deref(), Some("0 100 1000"));
        click(&mut harness, ".to-300");
        assert_eq!(harness.text_of(".seen").as_deref(), Some("300 100 1000"));
    }
}

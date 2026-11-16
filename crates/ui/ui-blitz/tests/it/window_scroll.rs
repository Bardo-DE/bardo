//! Window scrolling, by area: the wheel and page-key ease (design/11), what listeners hear of
//! fingers and zoom wheels, wheels over frames, platform feel, and a scroll offset read in the
//! frame that draws it. The harness runs the window's scroll step on its virtual clock.

mod eased {
    //! A window scrolls its containers by design/11's rules (the engine of `blitz_embed::scroll`), not
    //! by Blitz's own 20 px a line in one jump: a wheel detent is 60 px eased over at most 200 ms,
    //! detents accumulate, an element that takes the wheel (`data-wheel="capture"`) is left alone,
    //! the page key moves `max(0.8 v, v - 40)`, and a component that moves its own content can ask
    //! for the same ease of the detents it hears. The harness runs the window's scroll step before
    //! every frame it lays out, as the window loop does, on its virtual clock.

    use blitz_embed::element_id::ElementId;
    use blitz_embed::scroll::doc::{geom, scroller_by_id};
    use blitz_embed::scroll::geom::ScrollAxis;
    use dioxus::prelude::*;
    use std::cell::RefCell;
    use std::time::Duration;
    use ui_blitz::{ScrollAnimate, ScrollCmd, use_scroll_handle};
    use ui_components::host::gesture::{Gesture, WheelDelivery, use_gestures, use_gestures_with};
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    /// One frame, as the window's 120 Hz at its slowest.
    const FRAME: Duration = Duration::from_millis(8);

    thread_local! {
        /// The raw wheels the capturing element heard, and the gestures each kind of listener heard:
        /// per thread, as each test runs its harness on its own.
        static CAPTURED: RefCell<u32> = const { RefCell::new(0) };
        static AS_RECEIVED: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
        static EASED: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    }

    /// A list that scrolls 3000 px of content in 300 px, a capturing element beside it that takes the
    /// wheel itself, and listeners for the wheel's gestures.
    #[expect(non_snake_case, reason = "a Dioxus component function is named in CamelCase")]
    fn Page() -> Element {
        use_gestures(|gesture| {
            if let Gesture::Scroll { by, .. } = gesture {
                AS_RECEIVED.with(|heard| heard.borrow_mut().push(by.y.0));
            }
        });
        use_gestures_with(WheelDelivery::Eased, |gesture| {
            if let Gesture::Scroll { by, .. } = gesture {
                EASED.with(|heard| heard.borrow_mut().push(by.y.0));
            }
        });
        let handle = use_scroll_handle();
        rsx! {
            div { style: "position:relative; width:400px; height:400px; margin:0",
                div {
                    id: "list",
                    style: "position:absolute; left:0; top:0; width:200px; height:300px; overflow-y:auto",
                    div { style: "height:3000px", "rows" }
                }
                div {
                    id: "slider",
                    "data-wheel": "capture",
                    style: "position:absolute; left:220px; top:0; width:150px; height:300px; overflow-y:auto",
                    onwheel: |event| {
                        event.prevent_default();
                        CAPTURED.with(|count| *count.borrow_mut() += 1);
                    },
                    div { style: "height:3000px", "values" }
                }
                button {
                    id: "jump",
                    onclick: move |_| {
                        if let Some(handle) = &handle {
                            handle.send(ScrollCmd::To {
                                element: ElementId::new("list"),
                                axis: ScrollAxis::Y,
                                offset: 500.0,
                                animate: ScrollAnimate::Smooth,
                            });
                        }
                    },
                    style: "position:absolute; left:0; top:340px; width:80px; height:30px",
                    "jump"
                }
            }
        }
    }

    fn harness() -> Harness {
        CAPTURED.with(|count| *count.borrow_mut() = 0);
        AS_RECEIVED.with(|heard| heard.borrow_mut().clear());
        EASED.with(|heard| heard.borrow_mut().clear());
        Harness::new(Page, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
    }

    /// The vertical offset of the element with `id`.
    fn offset(harness: &Harness, id: &str) -> f64 {
        harness.with_doc(|doc| {
            let scroller = scroller_by_id(doc, &ElementId::new(id)).expect("laid out");
            geom(doc, scroller, ScrollAxis::Y)
                .expect("has geometry")
                .offset
                .0
        })
    }

    /// The offset after each of `frames` frames of 8 ms, starting with the one now.
    fn trace(harness: &mut Harness, id: &str, frames: usize) -> Vec<f64> {
        let mut seen = vec![offset(harness, id)];
        for _ in 0..frames {
            harness.advance(FRAME);
            seen.push(offset(harness, id));
        }
        seen
    }

    fn over(harness: &Harness, selector: &str) -> Point {
        harness.centre(selector).expect("laid out")
    }

    #[test]
    fn one_wheel_detent_moves_a_container_60_px_over_frames() {
        let mut harness = harness();
        let at = over(&harness, "#list");
        assert_eq!(offset(&harness, "list"), 0.0);
        // winit's sign: a click that scrolls the page down is -1.
        harness.send(Input::detents(at, 0.0, -1.0));
        let seen = trace(&mut harness, "list", 30);
        assert!(
            seen[0].abs() < 1e-6,
            "nothing moves before the first frame: {}",
            seen[0]
        );
        let moving = seen.iter().filter(|x| **x > 0.0 && **x < 60.0).count();
        assert!(moving >= 3, "intermediate frames, not a jump: {seen:?}");
        assert!(
            seen.windows(2).all(|pair| pair[1] >= pair[0]),
            "it only moves forward: {seen:?}"
        );
        let arrived = seen.iter().position(|x| *x == 60.0).expect("reaches 60 px");
        let by = Duration::from_millis(200);
        assert!(
            FRAME * u32::try_from(arrived).expect("small") <= by,
            "within 200 ms: frame {arrived}"
        );
        assert_eq!(*seen.last().expect("frames"), 60.0, "and stops there");
    }

    #[test]
    fn an_element_that_takes_the_wheel_is_left_alone_by_the_engine() {
        let mut harness = harness();
        let at = over(&harness, "#slider");
        harness.send(Input::detents(at, 0.0, -1.0));
        let seen = trace(&mut harness, "slider", 30);
        assert!(
            seen.iter().all(|x| *x == 0.0),
            "the engine did not scroll it: {seen:?}"
        );
        assert_eq!(offset(&harness, "list"), 0.0);
        assert_eq!(
            CAPTURED.with(|count| *count.borrow()),
            1,
            "it heard the raw wheel (and kept Blitz from scrolling)"
        );
    }

    #[test]
    fn page_down_moves_a_page_of_the_container_under_the_pointer() {
        let mut harness = harness();
        harness.send(Input::pointer_move(over(&harness, "#list")));
        harness.send(Input::key(ShortcutKey::PageDown));
        let seen = trace(&mut harness, "list", 30);
        // The list is 300 px tall: max(0.8 * 300, 300 - 40) = 260.
        assert_eq!(*seen.last().expect("frames"), 260.0);
        let moving = seen.iter().filter(|x| **x > 0.0 && **x < 260.0).count();
        assert!(moving >= 3, "eased over frames: {seen:?}");
        let arrived = seen.iter().position(|x| *x == 260.0).expect("260 px");
        assert!(
            FRAME * u32::try_from(arrived).expect("small") <= Duration::from_millis(208),
            "a page takes 200 ms at most: frame {arrived}"
        );
        harness.send(Input::key(ShortcutKey::Down));
        harness.advance(Duration::from_millis(100));
        assert_eq!(offset(&harness, "list"), 300.0, "an arrow is a 40 px line");
    }

    #[test]
    fn a_scroll_command_runs_through_the_engine_by_element_id() {
        let mut harness = harness();
        harness.send(Input::click(over(&harness, "#jump")));
        let seen = trace(&mut harness, "list", 40);
        assert_eq!(*seen.last().expect("frames"), 500.0);
        let moving = seen.iter().filter(|x| **x > 0.0 && **x < 500.0).count();
        assert!(moving >= 3, "smooth, not a jump: {seen:?}");
    }

    #[test]
    fn a_listener_that_asks_gets_the_detents_eased_and_the_others_get_them_whole() {
        let mut harness = harness();
        let at = over(&harness, "#list");
        harness.send(Input::detents(at, 0.0, -1.0));
        trace(&mut harness, "list", 40);
        assert_eq!(
            AS_RECEIVED.with(|heard| heard.borrow().clone()),
            vec![-60.0],
            "one detent, 60 px of content motion, in one event"
        );
        let eased = EASED.with(|heard| heard.borrow().clone());
        assert!(eased.len() >= 3, "a share per frame: {eased:?}");
        assert!(
            eased.iter().all(|by| *by < 0.0),
            "toward the detent: {eased:?}"
        );
        let total: f32 = eased.iter().sum();
        assert!(
            (total + 60.0).abs() < 1e-3,
            "summing to the detent: {total}"
        );
    }

    /// `clicks` wheel clicks `gap` apart, then the frames until everything has settled; the list's
    /// final offset.
    fn spun(clicks: u32, gap: Duration) -> f64 {
        let mut harness = harness();
        let at = over(&harness, "#list");
        for _ in 0..clicks {
            harness.send(Input::detents(at, 0.0, -1.0));
            harness.advance(gap);
        }
        harness.advance(Duration::from_millis(400));
        offset(&harness, "list")
    }

    #[test]
    fn a_slow_wheel_moves_60_px_a_click_and_a_spun_one_carries_each_click_further() {
        // Four clicks a second: not spun.
        let slow = spun(5, Duration::from_millis(250));
        assert!((slow - 300.0).abs() < 1.0, "{slow}");
        // Twenty clicks a second for a second: past the 1200 px unaccelerated.
        let end = spun(20, Duration::from_millis(48));
        assert!(end > 1800.0, "a spin of 20 clicks moved only {end}");
        // And a pause between spins starts afresh: one click back is 60 px again.
        let mut harness = harness();
        let at = over(&harness, "#list");
        for _ in 0..20 {
            harness.send(Input::detents(at, 0.0, -1.0));
            harness.advance(Duration::from_millis(48));
        }
        harness.advance(Duration::from_millis(600));
        let before = offset(&harness, "list");
        harness.send(Input::detents(at, 0.0, 1.0));
        harness.advance(Duration::from_millis(300));
        let step = before - offset(&harness, "list");
        assert!((step - 60.0).abs() < 1.0, "{step}");
    }
}

mod same_frame {
    //! A listener that moves its own content with the eased detents it hears (the PDF and text views)
    //! shows the move in the frame that handed it the detents. The window steps scrolling ahead of
    //! the frame's renders, so the listener's write is rendered, laid out and painted together;
    //! stepped after them, the content trailed the scroll by a frame. The harness runs the same
    //! order, and each `advance` of one frame here is one frame of the window.

    use dioxus::prelude::*;
    use std::cell::RefCell;
    use std::time::Duration;
    use ui_components::host::gesture::{Gesture, WheelDelivery, use_gestures_with};
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    const FRAME: Duration = Duration::from_millis(8);

    thread_local! {
        /// The px of content motion the listener has been handed so far.
        static HEARD: RefCell<f32> = const { RefCell::new(0.0) };
    }

    /// A bar as tall as the detents the listener has heard: content the app moves itself.
    #[expect(non_snake_case, reason = "a Dioxus component function is named in CamelCase")]
    fn Follower() -> Element {
        let mut moved = use_signal(|| 0.0_f32);
        use_gestures_with(WheelDelivery::Eased, move |gesture| {
            if let Gesture::Scroll { by, .. } = gesture {
                HEARD.with(|heard| *heard.borrow_mut() -= by.y.0);
                moved -= by.y.0;
            }
        });
        rsx! {
            div { id: "area", style: "width:300px; height:300px; margin:0",
                div { id: "bar", style: "width:20px; height:{moved}px; background:red" }
            }
        }
    }

    fn heard() -> f32 {
        HEARD.with(|heard| *heard.borrow())
    }

    #[test]
    fn what_a_listener_heard_this_frame_is_painted_in_this_frame() {
        HEARD.with(|heard| *heard.borrow_mut() = 0.0);
        let mut harness = Harness::new(
            Follower,
            HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
        );
        let at = harness.centre("#area").expect("laid out");
        harness.send(Input::detents(at, 0.0, -1.0));
        let mut seen = Vec::new();
        for _ in 0..30 {
            harness.advance(FRAME);
            let painted = harness.rect("#bar").map_or(0.0, |rect| rect.size.height.0);
            seen.push((heard(), painted));
        }
        assert!(
            seen.iter().any(|(heard, _)| *heard > 0.0),
            "the listener heard the detent: {seen:?}"
        );
        for (n, (heard, painted)) in seen.iter().enumerate() {
            assert!(
                (heard - painted).abs() <= 1.0,
                "frame {n}: the listener has heard {heard} px but the frame paints {painted}: {seen:?}"
            );
        }
    }
}

mod frames {
    //! A wheel or a touchpad over an `<iframe srcdoc>` scrolls the frame's own document by the
    //! engine's rules, eased like any scroller; at the frame's end the scroller around the frame
    //! takes over; and a wheel over nothing that scrolls goes on to the document.

    use blitz_embed::element_id::ElementId;
    use blitz_embed::scroll::doc::{geom, scroller_by_id};
    use blitz_embed::scroll::geom::ScrollAxis;
    use dioxus::prelude::*;
    use std::cell::RefCell;
    use std::time::Duration;
    use ui_components::host::gesture::GesturePhase;
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    const FRAME: Duration = Duration::from_millis(8);

    /// A frame whose document is 600 px tall in a 150 px window.
    const BODY: &str =
        "<html><body style='margin:0'><div style='height:600px'>text</div></body></html>";

    thread_local! {
        /// The raw wheels the plain box heard, per thread: each test runs its harness on its own.
        static HEARD: RefCell<u32> = const { RefCell::new(0) };
    }

    /// A 300 px scroller holding the frame and 1000 px more below it, and a box beside it that
    /// scrolls nothing but listens for the wheel.
    #[expect(non_snake_case, reason = "a Dioxus component function is named in CamelCase")]
    fn Page() -> Element {
        rsx! {
            div { style: "position:relative; width:380px; height:380px; margin:0",
                div {
                    id: "outer",
                    style: "position:absolute; left:0; top:0; width:300px; height:300px; overflow-y:auto",
                    iframe { srcdoc: BODY, style: "display:block; width:300px; height:150px; border:0" }
                    div { style: "height:1000px", "rows" }
                }
                div {
                    id: "plain",
                    style: "position:absolute; left:320px; top:0; width:70px; height:100px",
                    onwheel: |_| HEARD.with(|count| *count.borrow_mut() += 1),
                    "still"
                }
            }
        }
    }

    fn harness() -> Harness {
        HEARD.with(|count| *count.borrow_mut() = 0);
        Harness::new(Page, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
    }

    fn outer(harness: &Harness) -> f64 {
        harness.with_doc(|doc| {
            let scroller = scroller_by_id(doc, &ElementId::new("outer")).expect("laid out");
            geom(doc, scroller, ScrollAxis::Y)
                .expect("geometry")
                .offset
                .0
        })
    }

    /// How far the frame's document is scrolled.
    fn frame(harness: &Harness) -> f64 {
        harness.with_doc(|doc| {
            let iframe = doc
                .query_selector("iframe")
                .ok()
                .flatten()
                .expect("an iframe");
            let sub = doc
                .get_node(iframe)
                .and_then(|n| n.subdoc())
                .expect("a frame");
            sub.inner().viewport_scroll().y
        })
    }

    fn over_frame(harness: &Harness) -> Point {
        harness.centre("iframe").expect("laid out")
    }

    #[test]
    fn a_wheel_over_a_frame_scrolls_its_document_eased() {
        let mut harness = harness();
        let at = over_frame(&harness);
        harness.send(Input::detents(at, 0.0, -1.0));
        let mut seen = vec![frame(&harness)];
        for _ in 0..30 {
            harness.advance(FRAME);
            seen.push(frame(&harness));
        }
        let moving = seen.iter().filter(|y| **y > 0.0 && **y < 60.0).count();
        assert!(moving >= 3, "eased over frames, not a jump: {seen:?}");
        assert_eq!(*seen.last().expect("frames"), 60.0, "one detent is 60 px");
        assert_eq!(
            outer(&harness),
            0.0,
            "the frame took it, not the outer scroller"
        );
    }

    #[test]
    fn at_the_frames_end_the_outer_scroller_moves() {
        let mut harness = harness();
        let at = over_frame(&harness);
        // 600 - 150 = 450 px of frame to scroll; eight detents of 60 px carry it all the way.
        for _ in 0..8 {
            harness.send(Input::detents(at, 0.0, -1.0));
            harness.advance(Duration::from_millis(400));
        }
        assert_eq!(frame(&harness), 450.0, "the frame is at its end");
        assert_eq!(outer(&harness), 0.0, "and the outer scroller waited");
        harness.send(Input::detents(at, 0.0, -1.0));
        harness.advance(Duration::from_millis(400));
        assert_eq!(frame(&harness), 450.0);
        assert_eq!(outer(&harness), 60.0, "the next detent chained out");
    }

    #[test]
    fn a_wheel_or_fingers_over_nothing_that_scrolls_go_to_the_document() {
        let mut harness = harness();
        let at = harness.centre("#plain").expect("laid out");
        harness.send(Input::detents(at, 0.0, -1.0));
        assert_eq!(
            HEARD.with(|count| *count.borrow()),
            1,
            "the document heard it"
        );
        harness.send(Input::fingers(at, Px(0.0), Px(-20.0), GesturePhase::Began));
        assert_eq!(
            HEARD.with(|count| *count.borrow()),
            2,
            "and the fingers' too"
        );
        let on_frame = over_frame(&harness);
        harness.send(Input::detents(on_frame, 0.0, -1.0));
        assert_eq!(
            HEARD.with(|count| *count.borrow()),
            2,
            "a scroller took the other"
        );
    }
}

mod fingers {
    //! What a component that moves its own content hears of fingers on a touchpad and of a wheel
    //! under the primary modifier (`ui_components::host::gesture`): the fingers' run with its source and phases, the glide
    //! after a fast lift for a listener that asked for the ease, and a zoom wheel delivered whole.
    //! The harness runs the window's scroll step on its virtual clock, as the window loop does.

    use dioxus::prelude::*;
    use keymap::Platform;
    use std::cell::RefCell;
    use std::time::Duration;
    use ui_components::host::gesture::{
        Gesture, GesturePhase, ScrollSource, WheelDelivery, use_gestures, use_gestures_with,
    };
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    /// One scroll gesture a listener heard.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Heard {
        source: ScrollSource,
        phase: GesturePhase,
        y: f32,
        zoom: bool,
    }

    thread_local! {
        /// What each kind of listener heard, per thread: each test runs its harness on its own.
        static AS_RECEIVED: RefCell<Vec<Heard>> = const { RefCell::new(Vec::new()) };
        static EASED: RefCell<Vec<Heard>> = const { RefCell::new(Vec::new()) };
    }

    fn hear(log: &'static std::thread::LocalKey<RefCell<Vec<Heard>>>, gesture: Gesture) {
        if let Gesture::Scroll {
            source,
            phase,
            by,
            held,
            ..
        } = gesture
        {
            let zoom = !held.is_empty();
            let heard = Heard {
                source,
                phase,
                y: by.y.0,
                zoom,
            };
            log.with(|log| log.borrow_mut().push(heard));
        }
    }

    #[expect(non_snake_case, reason = "a Dioxus component function is named in CamelCase")]
    fn Page() -> Element {
        use_gestures(|gesture| hear(&AS_RECEIVED, gesture));
        use_gestures_with(WheelDelivery::Eased, |gesture| hear(&EASED, gesture));
        rsx! { div { style: "width:400px; height:400px" } }
    }

    fn harness() -> Harness {
        harness_on(keymap::Platform::Linux {
            desktop: keymap::Desktop::Ours,
        })
    }

    fn harness_on(platform: Platform) -> Harness {
        AS_RECEIVED.with(|log| log.borrow_mut().clear());
        EASED.with(|log| log.borrow_mut().clear());
        Harness::new(
            Page,
            HarnessConfig::new(VIEW)
                .with_platform(platform)
                .with_clock(Clock::Virtual),
        )
    }

    fn heard(log: &'static std::thread::LocalKey<RefCell<Vec<Heard>>>) -> Vec<Heard> {
        log.with(|log| log.borrow().clone())
    }

    const AT: Point = Point {
        x: Px(100.0),
        y: Px(100.0),
    };

    const FRAME: Duration = Duration::from_millis(8);

    /// Fingers moving `step` px down every frame for `moves` moves, then lifting with no last
    /// motion.
    fn flick(harness: &mut Harness, step: f32, moves: u32) {
        for n in 0..moves {
            let phase = match n {
                0 => GesturePhase::Began,
                _ => GesturePhase::Changed,
            };
            harness.send(Input::fingers(AT, Px(0.0), Px(step), phase));
            harness.advance(FRAME);
        }
        harness.send(Input::fingers(AT, Px(0.0), Px(0.0), GesturePhase::Ended));
    }

    fn phases(heard: &[Heard]) -> Vec<GesturePhase> {
        heard.iter().map(|h| h.phase).collect()
    }

    #[test]
    fn fingers_reach_a_listener_as_received_with_their_source_and_phases() {
        let mut harness = harness();
        flick(&mut harness, 30.0, 4);
        let got = heard(&AS_RECEIVED);
        assert_eq!(
            phases(&got),
            [
                GesturePhase::Began,
                GesturePhase::Changed,
                GesturePhase::Changed,
                GesturePhase::Changed,
                GesturePhase::Ended
            ]
        );
        assert!(
            got.iter().all(|h| h.source == ScrollSource::Finger),
            "{got:?}"
        );
        assert_eq!(got[0].y, 30.0, "the fingers' px, unchanged");
    }

    #[test]
    fn an_eased_listener_hears_the_glide_after_a_fast_lift_and_ends_when_it_stops() {
        let mut harness = harness();
        flick(&mut harness, 30.0, 6);
        let at_lift = heard(&EASED);
        assert_eq!(
            phases(&at_lift),
            [
                GesturePhase::Began,
                GesturePhase::Changed,
                GesturePhase::Changed,
                GesturePhase::Changed,
                GesturePhase::Changed,
                GesturePhase::Changed
            ],
            "the lift of a fast flick is not an end yet"
        );
        assert_eq!(
            heard(&AS_RECEIVED).last().map(|h| h.phase),
            Some(GesturePhase::Ended),
            "an as-received listener hears the lift at once"
        );

        let before = at_lift.len();
        for _ in 0..400 {
            harness.advance(FRAME);
        }
        let all = heard(&EASED);
        let glide = &all[before..];
        let (last, moving) = glide.split_last().expect("the glide spoke");
        assert_eq!(last.phase, GesturePhase::Ended, "{glide:?}");
        assert!(
            moving.iter().all(|h| h.phase == GesturePhase::Changed),
            "{glide:?}"
        );
        assert!(
            moving.len() > 10,
            "frame by frame, not a jump: {}",
            moving.len()
        );
        let distance: f32 = moving.iter().map(|h| h.y).sum();
        assert!(
            distance > 100.0,
            "a flick at 30 px a frame coasts on: {distance}"
        );
        assert!(
            moving
                .iter()
                .all(|h| h.source == ScrollSource::Finger && h.y > 0.0),
            "the glide goes the way the fingers went: {glide:?}"
        );
        assert_eq!(
            all.iter()
                .filter(|h| h.phase == GesturePhase::Ended)
                .count(),
            1,
            "one end, when the glide is over"
        );
    }

    #[test]
    fn a_wheel_under_the_primary_modifier_is_a_zoom_delivered_whole_never_eased() {
        let mut harness = harness();
        let zoom = |harness: &mut Harness| {
            harness.send(Input::detents_held(AT, 0.0, -1.0, Modifiers::SUPER));
        };
        zoom(&mut harness);
        zoom(&mut harness);
        let detent = |zoom: bool| Heard {
            source: ScrollSource::Wheel,
            phase: GesturePhase::Changed,
            y: 60.0,
            zoom,
        };
        let eased_now = heard(&EASED);
        assert_eq!(
            eased_now.len(),
            2,
            "one gesture per click, at once: {eased_now:?}"
        );
        assert!(
            eased_now
                .iter()
                .all(|h| h.zoom && h.y.abs() == detent(true).y)
        );
        assert_eq!(heard(&AS_RECEIVED), eased_now, "as-received hears the same");
        harness.advance(Duration::from_millis(400));
        assert_eq!(heard(&EASED), eased_now, "no eased frames follow a zoom");
    }

    /// The distance a glide of `step` px a frame (8 ms) covers, as an eased listener hears it.
    fn glide_of(step: f32) -> f32 {
        let mut harness = harness();
        flick(&mut harness, step, 8);
        let before = heard(&EASED).len();
        for _ in 0..600 {
            harness.advance(FRAME);
        }
        heard(&EASED)[before..].iter().map(|h| h.y).sum()
    }

    #[test]
    #[expect(clippy::cast_possible_truncation, reason = "layout and pointer lengths are far inside f32's exact range")]
    fn a_fast_lift_glides_further_than_its_own_speed_and_a_gentle_one_as_it_is() {
        use blitz_embed::scroll::config::ScrollSettings;
        use blitz_embed::scroll::engine::Physics;
        let glide = Physics::from_settings(&ScrollSettings::default()).glide;
        // name, px a frame (so px/s is 125 times it), the most the gain may add over the plain glide
        let cases = [
            ("gentle, 750 px/s", 6.0_f32, 1.0),
            ("fast, 3750 px/s", 30.0, 2.0),
        ];
        for (name, step, times) in cases {
            let speed = f64::from(step) * 125.0;
            let plain = glide.travel(speed, glide.duration(speed)) as f32;
            let got = glide_of(step);
            assert!(
                got >= plain * times * 0.9 && got <= plain * times * 1.1,
                "{name}: glided {got}, the plain glide is {plain}, expected about {times}x"
            );
        }
    }
}

mod native {
    //! Scrolling that feels like the platform it runs on, and a scroll offset a component reads in
    //! the frame that draws it.
    //!
    //! On a Mac the system follows the fingers' lift with momentum of its own, which winit reports as
    //! more touchpad events: the window applies it as it comes and adds no glide, boost or fling gain
    //! of its own, and a wheel click is the 40 px Chromium and `WebKit` use, with no acceleration on top
    //! of the system's. A component that windows over a scroller's offset (`VirtualList`) is handed
    //! the offset the frame draws, not the previous frame's.

    use blitz_embed::element_id::ElementId;
    use blitz_embed::scroll::doc::{geom, scroller_by_id};
    use blitz_embed::scroll::geom::ScrollAxis;
    use dioxus::prelude::*;
    use keymap::Platform;
    use std::time::Duration;
    use ui_blitz::{ScrollAnimate, ScrollCmd, use_scroll_handle};
    use ui_components::base::geometry::scroll::Scroll;
    use ui_components::host::gesture::GesturePhase;
    use ui_components::host::phase::Observe;
    use ui_components::prelude::*;
    use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

    const VIEW: Viewport = Viewport {
        width: 400,
        height: 400,
        scale_percent: 100,
    };

    thread_local! {
        static HANDLE: std::cell::RefCell<Option<ui_blitz::ScrollHandle>> = const { std::cell::RefCell::new(None) };
    }

    const FRAME: Duration = Duration::from_millis(8);

    const AT: Point = Point {
        x: Px(100.0),
        y: Px(150.0),
    };

    /// A list of 3000 px of content in 300, and what a component watching its scroll state reads.
    #[expect(non_snake_case, reason = "a Dioxus component function is named in CamelCase")]
    fn Page() -> Element {
        let host = use_document_host();
        let handle = use_scroll_handle();
        HANDLE.with(|slot| (*slot.borrow_mut()).clone_from(&handle));
        let seen = use_signal(|| None::<Scroll>);
        let mut watch = use_signal(|| None::<ui_components::host::phase::Watch>);
        let shown = seen().map_or_else(|| "none".to_owned(), |seen| format!("{}", seen.offset.0));
        rsx! {
            div {
                id: "list",
                style: "position:absolute; left:0; top:0; width:200px; height:300px; overflow-y:auto; scrollbar-width:none",
                onmounted: move |mounted: MountedEvent| {
                    let observed = host.geometry().observe(&mounted.data(), Observe::Scroll(seen));
                    if let ui_components::host::phase::Observed::Watching(found) = observed {
                        watch.set(Some(found));
                    }
                },
                div { style: "height:3000px", "rows" }
            }
            p { id: "seen", {shown} }
        }
    }

    fn harness(platform: Platform) -> Harness {
        Harness::new(
            Page,
            HarnessConfig::new(VIEW)
                .with_clock(Clock::Virtual)
                .with_platform(platform),
        )
    }

    fn offset(harness: &Harness) -> f64 {
        harness.with_doc(|doc| {
            let scroller = scroller_by_id(doc, &ElementId::new("list")).expect("laid out");
            geom(doc, scroller, ScrollAxis::Y)
                .expect("has geometry")
                .offset
                .0
        })
    }

    /// Let `time` pass a frame at a time, as a window's frames come (the virtual clock's advance
    /// steps only to the end, and an animation is a closed form of the time since it began).
    fn run_for(harness: &mut Harness, time: Duration) {
        for _ in 0..(time.as_millis() / FRAME.as_millis()) {
            harness.advance(FRAME);
        }
    }

    fn fingers(harness: &mut Harness, dy: f32, phase: GesturePhase) {
        harness.send(Input::fingers(AT, Px(0.0), Px(dy), phase));
        harness.advance(FRAME);
    }

    /// A flick of `steps` strokes of 30 px up the page (winit's sign: negative), lifted.
    #[expect(clippy::cast_precision_loss, reason = "a pixel or item count far below the float's exact integer range")]
    fn flick(harness: &mut Harness, steps: usize) -> f64 {
        for n in 0..steps {
            let phase = if n == 0 {
                GesturePhase::Began
            } else {
                GesturePhase::Changed
            };
            fingers(harness, -30.0, phase);
        }
        fingers(harness, 0.0, GesturePhase::Ended);
        30.0 * steps as f64
    }

    #[test]
    fn on_a_mac_the_systems_momentum_is_the_whole_tail() {
        let mut harness = harness(Platform::MacOs);
        let flicked = flick(&mut harness, 8);
        // The system's momentum: Started, Moved.., Ended, decaying pixel deltas.
        let tail: Vec<f32> = (0..40).map(|n| 14.0 * 0.92f32.powi(n)).collect();
        for (n, dy) in tail.iter().enumerate() {
            let phase = if n == 0 {
                GesturePhase::Began
            } else {
                GesturePhase::Changed
            };
            let before = offset(&harness);
            harness.send(Input::fingers(AT, Px(0.0), Px(-dy), phase));
            assert!(
                (offset(&harness) - before - f64::from(*dy)).abs() < 1e-3,
                "tail step {n} moves exactly its delta"
            );
            harness.advance(FRAME);
        }
        fingers(&mut harness, 0.0, GesturePhase::Ended);
        let want = flicked + tail.iter().map(|d| f64::from(*d)).sum::<f64>();
        run_for(&mut harness, Duration::from_secs(3));
        assert!(
            (offset(&harness) - want).abs() < 1e-2,
            "{} vs {want}",
            offset(&harness)
        );
    }

    #[test]
    fn on_a_mac_a_lift_the_system_sends_no_momentum_for_still_glides() {
        let mut harness = harness(Platform::MacOs);
        let flicked = flick(&mut harness, 8);
        run_for(&mut harness, Duration::from_secs(3));
        assert!(
            offset(&harness) > flicked + 100.0,
            "the engine glides when the system does not: {}",
            offset(&harness)
        );
    }

    #[test]
    fn a_mac_wheel_click_is_40_px_and_elsewhere_60() {
        // name, platform, clicks, the offset after the ease
        let cases = [
            ("a mac", Platform::MacOs, 40.0),
            (
                "elsewhere",
                Platform::Linux {
                    desktop: keymap::Desktop::Ours,
                },
                60.0,
            ),
        ];
        for (name, platform, want) in cases {
            let mut harness = harness(platform);
            harness.send(Input::detents(AT, 0.0, -1.0));
            run_for(&mut harness, Duration::from_millis(400));
            assert!(
                (offset(&harness) - want).abs() < 1e-6,
                "{name}: {}",
                offset(&harness)
            );
        }
    }

    #[test]
    fn a_mac_wheel_adds_no_acceleration_to_the_systems_own() {
        let mut harness = harness(Platform::MacOs);
        for _ in 0..30 {
            harness.send(Input::detents(AT, 0.0, -1.0));
            harness.advance(Duration::from_millis(10));
        }
        run_for(&mut harness, Duration::from_millis(400));
        assert!(
            (offset(&harness) - 30.0 * 40.0).abs() < 1e-6,
            "{}",
            offset(&harness)
        );
    }

    #[test]
    fn a_component_reads_the_offset_of_the_frame_that_draws_it() {
        let mut harness = harness(Platform::MacOs);
        harness.advance(Duration::from_millis(50));
        assert_eq!(harness.text_of("#seen").as_deref(), Some("0"));
        // A window's handle wakes a frame for a command; the harness's wakes nothing, so a key owes
        // the frame in which the queued jump runs.
        HANDLE.with(|slot| {
            slot.borrow()
                .as_ref()
                .expect("the page rendered")
                .send(ScrollCmd::To {
                    element: ElementId::new("list"),
                    axis: ScrollAxis::Y,
                    offset: 300.0,
                    animate: ScrollAnimate::Instant,
                });
        });
        harness.key_pending(ShortcutKey::Char('x'));
        let mut frames = Vec::new();
        while harness.step_frame() {
            frames.push((offset(&harness), harness.text_of("#seen")));
            assert!(frames.len() < 6, "frames never ran out: {frames:?}");
        }
        assert!(
            frames.iter().any(|(offset, _)| *offset == 300.0),
            "a frame draws the jump: {frames:?}"
        );
        for (offset, seen) in &frames {
            assert_eq!(
                seen.as_deref(),
                Some(format!("{offset}").as_str()),
                "every frame shows the offset it draws: {frames:?}"
            );
        }
    }
}

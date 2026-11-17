use std::sync::mpsc;

use shell_model::displays::ddc::debounce::Write;
use shell_model::displays::ddc::{Endpoint, Feature, Level};
use shell_model::ids::{DdcId, OutputName};
use tokio::sync::watch;

use super::{DdcDisplay, DdcJob, run};
use crate::displays::backend::ddc::retry;
use crate::env::ddc::{FakeBus, FakeState};

/// One MA270U-like display on bus 5 that answers brightness and volume only.
fn bus() -> FakeBus {
    let bus = FakeBus::default();
    let mut state = bus
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *state = FakeState {
        endpoints: vec![Endpoint {
            id: DdcId(5),
            connector: Some(OutputName("HDMI-A-2".to_owned())),
            product: "BenQ MA270U".to_owned(),
        }],
        ..FakeState::default()
    };
    state.values.insert(
        (DdcId(5), Feature::Brightness),
        Level {
            value: 49,
            max: 100,
        },
    );
    state.values.insert(
        (DdcId(5), Feature::SpeakerVolume),
        Level { value: 20, max: 50 },
    );
    drop(state);
    bus
}

fn start(
    bus: FakeBus,
) -> (
    mpsc::Sender<DdcJob>,
    watch::Receiver<Vec<DdcDisplay>>,
    std::thread::JoinHandle<()>,
) {
    let (jobs, rx) = mpsc::channel();
    let (tx, displays) = watch::channel(Vec::new());
    let thread = std::thread::spawn(move || run(std::sync::Arc::new(bus), &rx, tx));
    (jobs, displays, thread)
}

fn brightness(value: u16) -> DdcJob {
    DdcJob::Write(Write {
        id: DdcId(5),
        feature: Feature::Brightness,
        value,
    })
}

#[test]
fn a_probe_publishes_what_answers_and_nothing_else() {
    let fake = bus();
    let (jobs, displays, thread) = start(fake.clone());
    jobs.send(DdcJob::Probe).unwrap_or_else(|e| panic!("{e}"));
    drop(jobs);
    thread.join().unwrap_or_else(|_| panic!("worker panicked"));

    let displays = displays.borrow().clone();
    assert_eq!(displays.len(), 1);
    let controls = displays[0].controls;
    assert_eq!(
        controls.brightness,
        Some(Level {
            value: 49,
            max: 100
        })
    );
    assert_eq!(controls.volume, Some(Level { value: 20, max: 50 }));
    assert_eq!(
        controls.contrast, None,
        "unanswered: degraded, not an error"
    );
    assert_eq!(controls.gain, None);
    // Two answered reads, and three tries at each of the seven the display does not have.
    let reads = fake.0.lock().map_or(0, |s| s.reads);
    assert_eq!(reads, 2 + 7 * retry::ATTEMPTS);
}

#[test]
fn a_burst_of_writes_reaches_the_bus_once_with_the_latest_value() {
    let fake = bus();
    let (jobs, displays, thread) = start(fake.clone());
    jobs.send(DdcJob::Probe).unwrap_or_else(|e| panic!("{e}"));
    for value in [44, 38, 31, 25] {
        jobs.send(brightness(value))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    // The cache follows at once, before any write.
    let cached = || {
        displays
            .borrow()
            .first()
            .and_then(|d| d.controls.brightness)
    };
    base_testkit::eventually_blocking("the cache shows the latest value", || {
        cached()
            == Some(Level {
                value: 25,
                max: 100,
            })
    });

    // Once the quiet spell is over the bus has had one write, the latest value: the earlier
    // three of the burst never reached it.
    base_testkit::eventually_blocking("the debounced write reaches the bus", || {
        !fake.writes().is_empty()
    });
    assert_eq!(fake.writes(), vec![(DdcId(5), Feature::Brightness, 25)]);

    // A write to a feature the display does not have changes no cache and fails quietly.
    jobs.send(DdcJob::Write(Write {
        id: DdcId(5),
        feature: Feature::Contrast,
        value: 10,
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    jobs.send(brightness(49)).unwrap_or_else(|e| panic!("{e}"));
    drop(jobs);
    thread.join().unwrap_or_else(|_| panic!("worker panicked"));
    assert_eq!(
        fake.writes(),
        vec![
            (DdcId(5), Feature::Brightness, 25),
            (DdcId(5), Feature::Brightness, 49),
        ],
        "the pending restore is flushed on shutdown"
    );
}

#[test]
fn a_reread_follows_the_monitor_but_never_undoes_a_waiting_write() {
    let fake = bus();
    let set_on_the_monitor = |value| {
        let mut state = fake
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(level) = state.values.get_mut(&(DdcId(5), Feature::Brightness)) {
            level.value = value;
        }
    };
    let (jobs, displays, thread) = start(fake.clone());
    let send = |job| jobs.send(job).unwrap_or_else(|e| panic!("{e}"));
    let shown = |displays: &watch::Receiver<Vec<DdcDisplay>>| {
        displays
            .borrow()
            .first()
            .and_then(|d| d.controls.brightness)
            .map(|l| l.value)
    };
    send(DdcJob::Probe);
    // The monitor's own buttons: 49 -> 70; a re-read shows it.
    set_on_the_monitor(70);
    send(DdcJob::Reread(Feature::Brightness));
    base_testkit::eventually_blocking("the re-read shows the monitor's value", || {
        shown(&displays) == Some(70)
    });
    // bardo-shell's write of 30 waits for its debounce; a re-read meanwhile keeps 30 on show.
    send(brightness(30));
    send(DdcJob::Reread(Feature::Brightness));
    // The worker has served every job once it is joined: the re-read had its say by then.
    drop(jobs);
    thread.join().unwrap_or_else(|_| panic!("worker panicked"));
    assert_eq!(shown(&displays), Some(30));
    assert_eq!(fake.writes(), vec![(DdcId(5), Feature::Brightness, 30)]);
}

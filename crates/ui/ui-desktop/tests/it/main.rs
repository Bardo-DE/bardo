//! The probe against a private `dbus-daemon`: a fake service claims a name and releases it.
//! Nothing here reaches the real session or system bus; the guard kills the daemon by PID.

use base_testkit::PrivateBus;
use std::time::Duration;
use ui_desktop::{Capability, Desktop, Outputs};

const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn an_owned_name_is_here_and_releasing_it_flips_to_absent() {
    let bus = PrivateBus::start();
    let client = bus.connect().await;
    let service = bus.connect().await;

    let before = Desktop::probe_on(Some(&client), None).await;
    assert!(Capability::ALL.iter().all(|&c| !before.here(c)));

    service
        .request_name("org.bardo.Memory1")
        .await
        .expect("claims the name");
    let after = Desktop::probe_on(Some(&client), None).await;
    for capability in Capability::ALL {
        assert_eq!(after.here(capability), capability == Capability::Memory);
    }

    let mut watch = Desktop::watch_on(Some(&client), None).await;
    assert!(watch.current().here(Capability::Memory));
    service
        .release_name("org.bardo.Memory1")
        .await
        .expect("releases the name");
    let gone = tokio::time::timeout(WAIT, watch.changed())
        .await
        .expect("a change arrives")
        .expect("the feed is live");
    assert!(!gone.here(Capability::Memory));

    service
        .request_name("org.bardo.Memory1")
        .await
        .expect("claims it again");
    let back = tokio::time::timeout(WAIT, watch.changed())
        .await
        .expect("a change arrives")
        .expect("the feed is live");
    assert!(back.here(Capability::Memory));
}

#[tokio::test]
async fn an_activatable_name_is_here_before_anyone_owns_it() {
    let bus = PrivateBus::start_activatable(&["org.bardo.Intents1"]);
    let client = bus.connect().await;
    let desktop = Desktop::probe_on(Some(&client), None).await;
    assert!(desktop.here(Capability::Intents));
    assert!(!desktop.here(Capability::Memory));
}

#[tokio::test]
async fn a_missing_bus_leaves_its_capabilities_absent() {
    let bus = PrivateBus::start();
    let session = bus.connect().await;
    let desktop = Desktop::probe_on(Some(&session), None).await;
    assert!(!desktop.here(Capability::Helpers));
    assert!(
        Desktop::watch_on(None, None)
            .await
            .changed()
            .await
            .is_none()
    );
}

#[tokio::test]
async fn the_system_bus_carries_packagekit() {
    let session_bus = PrivateBus::start();
    let system_bus = PrivateBus::start();
    let session = session_bus.connect().await;
    let system = system_bus.connect().await;
    let daemon = system_bus.connect().await;
    // A name on the session bus does not count as PackageKit.
    let _decoy = session_bus.connect().await;
    daemon
        .request_name("org.freedesktop.PackageKit")
        .await
        .expect("claims the name");
    let desktop = Desktop::probe_on(Some(&session), Some(&system)).await;
    assert!(desktop.here(Capability::Helpers));
}

type Row = (u32, String, (i32, i32, u32, u32), (i32, i32, u32, u32), u32);

/// A stand-in for the shell's `org.bardo.Outputs1`.
struct FakeOutputs(Vec<Row>);

#[zbus::interface(name = "org.bardo.Outputs1")]
impl FakeOutputs {
    #[zbus(property)]
    fn work_areas(&self) -> Vec<Row> {
        self.0.clone()
    }
}

fn row(id: u32, top: u32) -> Row {
    (
        id,
        format!("DP-{id}"),
        (0, 0, 2560, 1440),
        (0, i32::try_from(top).unwrap_or(0), 2560, 1440 - top),
        180,
    )
}

#[tokio::test]
async fn the_outputs_are_read_and_a_change_is_followed() {
    use ui_desktop::{Outputs, OutputsWatch};

    let bus = PrivateBus::start();
    let client = bus.connect().await;
    let shell = bus.connect().await;

    // No shell: the portable fallback, an empty list.
    assert!(Outputs::read_on(&client).await.areas().is_empty());
    assert!(
        !Desktop::probe_on(Some(&client), None)
            .await
            .here(Capability::Outputs)
    );

    shell
        .object_server()
        .at("/org/bardo/Outputs1", FakeOutputs(vec![row(1, 32)]))
        .await
        .expect("exports the object");
    dbus_kit::ready::serving(&shell)
        .await
        .expect("the object server answers");
    shell
        .request_name("org.bardo.Outputs1")
        .await
        .expect("claims the name");
    assert!(
        Desktop::probe_on(Some(&client), None)
            .await
            .here(Capability::Outputs)
    );

    let read = Outputs::read_on(&client).await;
    assert_eq!(read.areas().len(), 1);
    assert_eq!(read.areas()[0].work.y, 32);
    assert_eq!(read.areas()[0].physical(), (3840, 2160));

    let mut watch = OutputsWatch::on(&client).await.expect("follows");
    assert_eq!(watch.current().await.areas().len(), 1);
    let iface = shell
        .object_server()
        .interface::<_, FakeOutputs>("/org/bardo/Outputs1")
        .await
        .expect("the object");
    iface.get_mut().await.0 = vec![row(1, 32), row(2, 0)];
    iface
        .get()
        .await
        .work_areas_changed(iface.signal_emitter())
        .await
        .expect("announces");
    let after = tokio::time::timeout(WAIT, watch.changed())
        .await
        .expect("a change arrives")
        .expect("the feed is live");
    assert_eq!(after.areas().len(), 2);
}

/// What a process holding `org.bardo.Outputs1` says when asked for the work areas.
#[derive(Clone)]
enum Answer {
    Rows(Vec<Row>),
    /// It holds the name and never replies.
    Never,
}

struct Shell(Answer);

#[zbus::interface(name = "org.bardo.Outputs1")]
impl Shell {
    #[zbus(property)]
    async fn work_areas(&self) -> Vec<Row> {
        match &self.0 {
            Answer::Rows(rows) => rows.clone(),
            Answer::Never => std::future::pending().await,
        }
    }
}

/// A connection on `bus` that holds `org.bardo.Outputs1` and answers as `answer` says.
async fn hold_outputs(bus: &PrivateBus, answer: Answer) -> zbus::Connection {
    let holder = bus.connect().await;
    holder
        .object_server()
        .at("/org/bardo/Outputs1", Shell(answer))
        .await
        .expect("served");
    dbus_kit::ready::serving(&holder)
        .await
        .expect("the object server answers");
    holder
        .request_name("org.bardo.Outputs1")
        .await
        .expect("claims the name");
    holder
}

fn row_at(id: u32, work: (i32, i32, u32, u32), scale: u32) -> Row {
    (id, format!("DP-{id}"), (0, 0, 1920, 1080), work, scale)
}

#[tokio::test]
async fn rows_that_cannot_hold_a_window_read_as_no_outputs() {
    let bus = PrivateBus::start();
    let rows = vec![
        row_at(1, (0, 0, 1920, 0), 120),
        row_at(2, (0, 32, 1920, 1048), 0),
        row_at(3, (0, 32, 1920, 1048), 120),
    ];
    let _holder = hold_outputs(&bus, Answer::Rows(rows)).await;
    let client = bus.connect().await;
    let outputs = Outputs::read_on(&client).await;
    let ids: Vec<u32> = outputs.areas().iter().map(|area| area.id).collect();
    assert_eq!(ids, [3]);
}

#[tokio::test]
async fn a_shell_that_holds_the_name_and_never_answers_reads_as_no_outputs() {
    let bus = PrivateBus::start();
    let _holder = hold_outputs(&bus, Answer::Never).await;
    let client = bus.connect().await;
    let started = std::time::Instant::now();
    let outputs = tokio::time::timeout(Duration::from_secs(15), Outputs::read_on(&client))
        .await
        .expect("the read ends by its own deadline");
    assert!(outputs.areas().is_empty());
    assert!(started.elapsed() < Duration::from_secs(10));
}

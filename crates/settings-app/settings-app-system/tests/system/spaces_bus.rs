//! The real Spaces link over a registry and a memory on a private bus: the list is read, memory
//! is told the choice before the registry is asked, a removal with no memory running goes on only
//! when it keeps everything, and nothing is half done.

use crate::private_bus::PrivateBus;
use crate::spaces_fakes::serve;
use memory_core::{HistoryFate, MemoryFate, Removal};
use settings_app_model::SpaceKey;
use settings_app_system::bus::{BusSource, Vouch};
use settings_app_system::spaces::{OnBus, SpacesLink};
use settings_dbus::AnyOwner;
fn link(bus: &PrivateBus) -> OnBus {
    OnBus(BusSource::Address(bus.address.clone(), Vouch::of(AnyOwner)))
}

/// The registry's Spaces.
const SPACES: [(&str, &str); 2] = [("space-1", "Trip"), ("space-2", "Work")];

const KEEP: Removal = Removal {
    memories: MemoryFate::MoveToApps,
    history: HistoryFate::Keep,
};

fn trip() -> SpaceKey {
    SpaceKey("space-1".to_owned())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_removal_tells_memory_first_and_then_asks_the_registry() {
    let bus = PrivateBus::start("spaces-order");
    let fakes = serve(&bus, &SPACES, true).await;
    let link = link(&bus);
    link.remove(trip(), KEEP).await.expect("removed");
    assert_eq!(
        fakes.asked(),
        [
            r#"memory space-1 {"memories":"move_to_apps","history":"keep"}"#.to_owned(),
            "registry remove space-1".to_owned(),
        ]
    );
    let left = link.list().await.expect("the registry answers");
    assert_eq!(left.len(), 1, "the removed Space is gone from the list");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_no_memory_a_delete_choice_removes_nothing_and_says_why() {
    for (name, removal) in [
        (
            "spaces-no-memory-memories",
            Removal {
                memories: MemoryFate::Delete,
                history: HistoryFate::Keep,
            },
        ),
        (
            "spaces-no-memory-history",
            Removal {
                memories: MemoryFate::MoveToApps,
                history: HistoryFate::Delete,
            },
        ),
    ] {
        let bus = PrivateBus::start(name);
        let fakes = serve(&bus, &SPACES, false).await;
        let ended = link(&bus).remove(trip(), removal).await;
        let words = ended.expect_err("a delete choice waits for memory");
        assert!(words.contains("Memory"), "{name}: {words}");
        assert!(
            fakes.asked().is_empty(),
            "{name}: the registry was not asked, so nothing is half done"
        );
    }
}

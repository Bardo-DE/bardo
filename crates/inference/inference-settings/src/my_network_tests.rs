//! `ai.attached.my_network`: whether the person's other machines count as this computer for a floor
//! that says "this computer".

use super::*;
use account_capability::Locality;
use account_core::DataClass;

#[derive(serde::Deserialize, Default)]
struct File {
    #[serde(default)]
    ai: AiConfig,
}

fn settings(text: &str) -> (Settings, Vec<String>) {
    let config = toml::from_str::<File>(text).expect("config");
    let resolved = resolve(&config.ai);
    (resolved.settings, resolved.rejected)
}

#[test]
fn the_row_is_off_by_default_and_reads_on_and_names_a_value_it_does_not_know() {
    assert!(!settings("").0.my_network);
    let (on, rejected) = settings("[ai.attached]\nmy_network = true\n");
    assert_eq!((on.my_network, rejected), (true, vec![]));
    let (off, rejected) = settings("[ai.attached]\nmy_network = false\n");
    assert_eq!((off.my_network, rejected), (false, vec![]));
    let (odd, rejected) = settings("[ai.attached]\nmy_network = \"sometimes\"\n");
    assert!(!odd.my_network);
    assert_eq!(rejected, [MY_NETWORK.to_owned()]);
}

#[test]
fn on_lets_an_on_device_floor_reach_my_machines_and_nothing_else_moves() {
    let text = "[ai]\nlocal_only = false\n[ai.floor]\nmail = \"on_device\"\nnotes = \"local_network\"\n\
                files = \"anywhere\"\n[ai.attached]\nmy_network = true\n";
    let (on, _) = settings(text);
    let (off, _) = settings(&text.replace("my_network = true", "my_network = false"));
    let machines = Locality::LocalNetwork;
    let cloud = Locality::Cloud { region: None };
    let table = [
        // class, floor without the row, floor with it
        (DataClass::Mail, Floor::OnDevice, Floor::LocalNetwork),
        (DataClass::Notes, Floor::LocalNetwork, Floor::LocalNetwork),
        (DataClass::Files, Floor::Anywhere, Floor::Anywhere),
        // The proposed default of a class the file does not mention.
        (DataClass::Photos, Floor::OnDevice, Floor::LocalNetwork),
        (DataClass::Public, Floor::Anywhere, Floor::Anywhere),
    ];
    for (class, without, with) in table {
        assert_eq!(off.routing_policy().floor(class), without, "{class:?} off");
        assert_eq!(on.routing_policy().floor(class), with, "{class:?} on");
        // The floor the person set is untouched: only routing reads the other one.
        assert_eq!(on.policy.floor(class), without, "{class:?} as set");
    }
    // Another machine of theirs passes only where the row says so; the cloud never does.
    assert!(
        !off.routing_policy()
            .floor(DataClass::Mail)
            .admits(&machines)
    );
    assert!(on.routing_policy().floor(DataClass::Mail).admits(&machines));
    assert!(!on.routing_policy().floor(DataClass::Mail).admits(&cloud));
    assert!(!on.routing_policy().local_only);
    assert_eq!(off.routing_policy(), off.policy);
}

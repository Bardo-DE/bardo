//! Reloading the configuration file puts the rows in force: what `Engines` routes by changes with
//! it (the rows' own meaning is `inference-settings`').

use super::*;
use crate::config::InferdConfig;
use crate::engines::Engines;
use crate::peers::Role;
use crate::supervise::{Ports, Supervised};
use crate::testkit::{Recorder, models};
use account_capability::{LlmFeature, Tier};
use account_core::DataClass;
use account_match::LlmNeed;
use account_match::Need;
use base_ids::Tokens;
use base_testkit::Scratch;
use engine_supervisor::{FakeGpu, FakeReadyProbe, GpuMemory, Probe, SupervisorConfig};
use inference_core::Policy;
use inference_router::session::SessionSpec;
use model_catalog::MiB;
use std::time::Duration;

fn need() -> Need {
    Need::Llm(LlmNeed::new([LlmFeature::Chat].into(), Tokens(1000)))
}

fn engines(scratch: &Scratch) -> Engines {
    let models = models(scratch);
    let supervised = Supervised::start(
        models.iter().map(|m| m.spec.clone()).collect(),
        SupervisorConfig::default(),
        Ports {
            host: Recorder::default(),
            probe: FakeReadyProbe(Probe::Ready),
            gpu: FakeGpu(GpuMemory {
                total: MiB(16_000),
                used_by_others: MiB(0),
            }),
        },
    );
    let pool = models
        .iter()
        .map(inference_router::LocalModel::model_ref)
        .collect();
    Engines::new(
        models,
        supervised,
        Policy::proposed(),
        inference_core::TierMap::automatic(inference_core::Slot::Text, pool),
    )
}

fn spec() -> SessionSpec {
    SessionSpec {
        need: need(),
        class: DataClass::Notes,
        tier: Tier::Balanced,
        usage: account_core::consent::Usage::Interactive,
    }
}

#[tokio::test]
async fn show_reason_off_stops_the_answers_saying_why_and_a_reload_applies_it() {
    let scratch = Scratch::new("settings-show");
    let engines = engines(&scratch);
    let file = ConfigFile::new(scratch.path().join("inferd.toml"));
    let reload = Reload::new(file.clone(), engines.clone());
    let show = || {
        engines
            .route_detailed(&spec(), Role::App)
            .expect("route")
            .0
            .show
    };
    assert!(show());
    file.set("ai.model.text.balanced", "auto".into())
        .expect("write");
    let everything = engines
        .listed()
        .iter()
        .map(|one| toml::Value::String(format!("{}/{}", one.card.account, one.card.model)))
        .collect();
    file.set("ai.auto.pool.text", toml::Value::Array(everything))
        .expect("write");
    file.set("ai.auto.show_reason", false.into())
        .expect("write");
    // Nothing is in force until the file is read again.
    assert!(show());
    assert_eq!(reload.now().expect("reload").rejected, Vec::<String>::new());
    assert!(!show());
    file.set("ai.auto.show_reason", true.into()).expect("write");
    reload.now().expect("reload");
    assert!(show());
}

#[tokio::test]
async fn a_reload_changes_the_route_of_the_next_session_and_a_bad_file_changes_nothing() {
    let scratch = Scratch::new("settings-reload");
    let engines = engines(&scratch);
    let file = ConfigFile::new(scratch.path().join("inferd.toml"));
    let reload = Reload::new(file.clone(), engines.clone());
    let route = || {
        engines
            .route_detailed(&spec(), Role::App)
            .map(|(r, _)| r.served.model)
    };
    route().unwrap();
    // A named model that is not here: refused, naming it.
    file.set("ai.model.text.balanced", "local/ghost".into())
        .expect("write");
    reload.now().expect("reload");
    route().unwrap_err();
    // A file that does not read keeps what is in force.
    std::fs::write(file.path(), "[ai\nbroken").expect("write");
    reload.now().unwrap_err();
    route().unwrap_err();
    let _ = file.set("ai.model.text.balanced", "".into());
    std::fs::write(
        file.path(),
        "[ai.model.text]\nbalanced = \"local/tiny-chat\"\n",
    )
    .expect("write");
    reload.now().expect("reload");
    assert_eq!(route().expect("served").as_str(), "tiny-chat");
}

#[tokio::test]
async fn the_watch_notices_a_change_made_by_someone_else() {
    let scratch = Scratch::new("settings-watch");
    let engines = engines(&scratch);
    let file = ConfigFile::new(scratch.path().join("inferd.toml"));
    let watch = Reload::new(file.clone(), engines.clone()).watch(Duration::from_millis(20));
    let show = || engines.settings().auto.show_reason;
    assert!(show());
    std::fs::write(file.path(), "[ai.auto]\nshow_reason = false\n").expect("write");
    base_testkit::eventually("the change was noticed", || !show()).await;
    watch.abort();
    assert!(!show());
}

#[test]
fn set_writes_one_row_at_its_path_and_keeps_the_rest() {
    let scratch = Scratch::new("settings-set");
    let file = ConfigFile::new(scratch.path().join("sub").join("inferd.toml"));
    assert_eq!(
        file.read().expect("missing is the default"),
        InferdConfig::default()
    );
    file.set("ai.model.text.fast", "auto".into())
        .expect("write");
    file.set("ai.model.text.best", "local/beta".into())
        .expect("write");
    file.set("ai.auto.allow_evict", "never".into())
        .expect("write");
    let read = file.read().expect("reads");
    assert_eq!(read.ai.model["text"]["fast"], "auto");
    assert_eq!(read.ai.model["text"]["best"], "local/beta");
    assert_eq!(read.ai.auto.allow_evict.as_deref(), Some("never"));
    // rel-8: the write is synced and renamed, and nothing is left staged. (A crash that loses an
    // unsynced file cannot be made in a test; this keeps the synced path working.)
    let staged = std::fs::read_dir(file.path().parent().expect("dir"))
        .expect("dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(staged, 0);
    // A file that does not parse is left as it was.
    std::fs::write(file.path(), "[ai\n").expect("write");
    assert!(file.set("ai.local_only", false.into()).is_err());
    assert_eq!(std::fs::read_to_string(file.path()).expect("read"), "[ai\n");
}

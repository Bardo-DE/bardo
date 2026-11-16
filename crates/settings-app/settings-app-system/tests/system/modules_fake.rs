//! The in-memory modules link: a module hosted in the process is found again after it goes away and
//! comes back. Opening, refusing and changing are tested over the bus in `modules_bus`.

use crate::modules_bus::{ROW, Words, daemon, is_text};
use settings_app_model::Absence;
use settings_app_system::modules::{FakeModules, Followed, ModulesLink};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use ui_settings::live::Verdict;
use ui_settings::schema::KeyPath;

/// The next thing a follower is told, or a failure when nothing comes.
async fn next(heard: &mut UnboundedReceiver<Followed>) -> Followed {
    heard.recv().await.expect("the follower is still there")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_module_that_goes_away_and_comes_back_is_read_again() {
    let modules = FakeModules::new();
    let daemon = daemon();
    let (told, mut heard) = unbounded_channel();
    let following = modules.follow(&daemon, is_text, told);
    let checking = async {
        assert!(matches!(
            next(&mut heard).await,
            Followed::Gone(Absence::NotRunning)
        ));
        modules.host(&daemon, Words::holding("first", Verdict::Allow));
        assert!(matches!(next(&mut heard).await, Followed::Opened(..)));
        modules.away(&daemon, Absence::Unavailable);
        assert!(matches!(
            next(&mut heard).await,
            Followed::Gone(Absence::Unavailable)
        ));
        modules.host(&daemon, Words::holding("second", Verdict::Allow));
        let Followed::Opened(_, described) = next(&mut heard).await else {
            panic!("expected the module to come back");
        };
        let row = KeyPath(ROW.to_owned());
        assert_eq!(described.values.get(&row), Some(&"second".to_owned()));
    };
    tokio::select! {
        () = following => panic!("the follower ended"),
        () = checking => {}
    }
}

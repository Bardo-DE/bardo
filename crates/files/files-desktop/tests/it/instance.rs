//! One running app: the first launch owns the names, a second hands its request on.

use files_core::Location;
use files_desktop::file_manager::INTERFACE;
use files_desktop::instance::APP_NAME;
use files_desktop::{Claim, InstanceError, Show, ShowRequest, StartupId, claim, forward};
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

use crate::bus::PrivateBus;
use crate::file_manager::{folder, next, reveal, sink};

fn shows() -> Vec<Show> {
    vec![
        Show {
            request: ShowRequest::Items(vec![
                reveal("/tmp/My Docs", "a b.txt"),
                reveal("/home/me", "café #1.pdf"),
            ]),
            startup: StartupId::new("tok-1"),
        },
        Show {
            request: ShowRequest::Folders(vec![
                Location::Trash,
                Location::Folder(folder("/srv/data")),
            ]),
            startup: None,
        },
        Show {
            request: ShowRequest::Properties(vec![reveal("/tmp", "x")]),
            startup: StartupId::new("tok-2"),
        },
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_second_claim_finds_the_first_running_and_forwards_to_it() {
    let bus = PrivateBus::start("claim_twice");
    let (first_sink, mut first_heard) = sink();
    let first = bus.connect().await;
    let Claim::First(served) = claim(&first, first_sink).await.unwrap() else {
        panic!("the first claim must be the app");
    };
    assert!(served.owns_file_manager());

    let (second_sink, mut second_heard) = sink();
    let second = bus.connect().await;
    assert!(matches!(
        claim(&second, second_sink).await.unwrap(),
        Claim::Running
    ));

    for show in shows() {
        forward(&second, &show).await.unwrap();
        assert_eq!(next(&mut first_heard).await, show);
    }
    // The launch that found one running serves nothing itself.
    second_heard.try_recv().unwrap_err();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_file_manager_keeps_its_name_and_the_app_still_runs() {
    let bus = PrivateBus::start("name_taken");
    let other = bus.connect().await;
    other.request_name(INTERFACE).await.unwrap();

    let (sink, mut heard) = sink();
    let app = bus.connect().await;
    let Claim::First(served) = claim(&app, sink).await.unwrap() else {
        panic!("the app must run");
    };
    assert!(!served.owns_file_manager());

    let owner = DBusProxy::new(&app)
        .await
        .unwrap()
        .get_name_owner(BusName::try_from(INTERFACE).unwrap())
        .await
        .unwrap();
    assert_eq!(
        Some(owner.as_str()),
        other.unique_name().map(|n| n.as_str())
    );

    // A later launch still reaches the app by its own name.
    let later = bus.connect().await;
    let show = shows().remove(0);
    forward(&later, &show).await.unwrap();
    assert_eq!(next(&mut heard).await, show);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forwarding_with_no_app_running_says_so() {
    let bus = PrivateBus::start("not_running");
    let client = bus.connect().await;
    let result = forward(&client, &shows().remove(0)).await;
    assert!(
        matches!(result, Err(InstanceError::NotRunning)),
        "{result:?}"
    );
    assert_eq!(APP_NAME, "org.bardo.Files");
}

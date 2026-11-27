//! The desktop-wide Spaces through the client (`Spaces`), against the real
//! accountd front end on a private bus: list, create, the stream of changes, and the refusals
//! read back as `SpacesError`s.

use super::common;

use account_client::{LinkError, SpacesError};
use account_client_dbus::Bus;
use account_core::{SpaceChange, SpaceLook, SpaceName};
use common::accountd::{Daemon, named, photos};

fn name(text: &str) -> SpaceName {
    SpaceName::parse(text).expect("name")
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_lists_creates_and_hears_of_new_spaces() {
    use account_dbus::BusStream;
    let daemon = Daemon::start([]).await;
    let watcher = Bus::over(daemon.client_as(named("org.bardo.Mail")).await).spaces();
    let mut changes = watcher.watch().await.expect("watch");
    assert_eq!(watcher.list().await.expect("list"), [] as [account_core::DesktopSpaceRecord; 0]);

    let maker = Bus::over(daemon.client_as(photos()).await).spaces();
    let look = SpaceLook::parse(r#"{"colour":"teal"}"#).expect("look");
    let id = maker.create(&name("Work"), &look).await.expect("create");

    let list = watcher.list().await.expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, id);
    assert_eq!(list[0].name, name("Work"));
    assert_eq!(list[0].look, look);

    let next = tokio::time::timeout(
        base_testkit::GENEROUS,
        std::future::poll_fn(|cx| std::pin::Pin::new(&mut changes).poll_next(cx)),
    )
    .await
    .expect("told in time")
    .expect("a change")
    .expect("well formed");
    assert_eq!(next, (id, SpaceChange::Created));
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_new_spaces_and_a_stranger_are_told_apart() {
    let daemon = Daemon::start([]).await;
    let app = Bus::over(daemon.client().await).spaces();
    for n in 0..account_spaces::CREATES_PER_WINDOW {
        app.create(&name(&format!("Space {n}")), &SpaceLook::default())
            .await
            .expect("create");
    }
    assert_eq!(
        app.create(&name("One more"), &SpaceLook::default()).await,
        Err(SpacesError::TooMany)
    );

    let stranger = Bus::over(daemon.stranger().await).spaces();
    assert!(matches!(
        stranger.list().await,
        Err(SpacesError::Link(LinkError::Denied))
    ));
}

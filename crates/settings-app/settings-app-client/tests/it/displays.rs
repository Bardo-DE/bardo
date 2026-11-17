//! The Displays flow: a brightness the shell cannot set is said to the person, and the slider
//! moves where it was dragged meanwhile.

use crate::support::builder;
use settings_app_client::{Heard, Notice, Pane};
use settings_app_model::{Brightness, OutputId};
use settings_app_system::displays::FakeDisplays;
use std::sync::Arc;

#[tokio::test]
async fn a_brightness_that_cannot_be_set_is_told_to_the_person() {
    let link = FakeDisplays::missing(settings_app_model::Absence::NotRunning);
    let client = builder().with_displays(Arc::new(link)).build();
    let displays = client.displays();

    displays
        .set_brightness(OutputId("eDP-1".to_owned()), Brightness(40))
        .await;

    let events = displays.live().events();
    let Some(heard) = events.next().await else {
        panic!("the pane tells the person");
    };
    assert!(
        matches!(heard, Heard::Notice(Notice::Words(_))),
        "{heard:?}"
    );
}

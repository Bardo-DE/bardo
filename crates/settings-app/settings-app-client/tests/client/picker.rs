//! The model picker's first state: with no assistant to look at, it is unavailable.

use crate::support::builder;
use settings_app_client::Pane;
use settings_app_model::{Absence, PickerInput, PickerState};

#[tokio::test]
async fn a_picker_with_no_assistant_is_unavailable_and_says_why() {
    let client = builder().build();
    let picker = client.picker();

    picker
        .send(PickerInput::Found(Err(Absence::NotRunning)))
        .await;

    assert!(matches!(
        picker.live().state(),
        PickerState::Unavailable(Absence::NotRunning)
    ));
}

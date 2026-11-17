//! The sync service on the bus, as the Accounts pane's link.

use super::holds::{confirm_discard, follow_holds, hold_now, sync_now};
use super::status::sync_status;
use settings_app_link::bus::BusSource;
use settings_app_link::sync::SyncLink;
use settings_app_model::{DatasetName, HoldInput, SyncNowEnd, SyncStatus, Work};
use tokio::sync::mpsc::UnboundedSender;

/// The sync service on the bus it is built over.
#[derive(Debug)]
pub struct OnBus(pub BusSource);

impl SyncLink for OnBus {
    fn status(&self, dataset: DatasetName) -> Work<'_, Option<SyncStatus>> {
        Box::pin(sync_status(&self.0, dataset))
    }

    fn confirm_discard(&self, dataset: DatasetName) -> Work<'_, Option<HoldInput>> {
        Box::pin(confirm_discard(&self.0, dataset))
    }

    fn hold_now(&self, dataset: DatasetName) -> Work<'_, HoldInput> {
        Box::pin(hold_now(&self.0, dataset))
    }

    fn sync_now(&self, dataset: DatasetName) -> Work<'_, SyncNowEnd> {
        Box::pin(sync_now(&self.0, dataset))
    }

    fn follow_holds(&self, told: UnboundedSender<HoldInput>) -> Work<'_, ()> {
        Box::pin(follow_holds(&self.0, move |input| {
            let _gone = told.send(input);
        }))
    }
}

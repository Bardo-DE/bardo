//! "Sync now": the dataset a sync row names.

use settings_app_model::{AccountKey, DatasetName};
use settings_types::schema::KeyPath;

#[test]
fn a_sync_row_names_its_dataset_by_the_accounts_bus_segment() {
    let row = KeyPath("accounts.work.example.sync.photos".to_owned());
    assert_eq!(
        DatasetName::of_switch(&AccountKey("work.example".to_owned()), &row),
        DatasetName("work_example/photos".to_owned())
    );
}

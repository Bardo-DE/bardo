use super::*;
use crate::testing::scratch;
use base_ids::Bytes;
use base_time::UnixSeconds;
use sync_core::{
    Acknowledgement, Anchor, BaseVersion, Conflict, ContentHash, ItemState, RemoteSide,
    RemoteVersion, Tombstone, TombstoneOrigin,
};

fn item(local: &str, remote: Option<&str>, state: ItemState) -> JournalItem {
    JournalItem {
        local: LocalId(local.into()),
        remote: remote.map(|r| RemoteId(r.into())),
        path: ItemPath(format!("dir/{local}")),
        size: Bytes(7),
        hash: Some(ContentHash("abc".into())),
        remote_version: remote.map(|_| RemoteVersion("v1".into())),
        base: remote.map_or(BaseVersion::Absent, |_| {
            BaseVersion::At(RemoteVersion("v1".into()))
        }),
        state,
    }
}

fn tombstone(id: &str, ack: Acknowledgement) -> StoredTombstone {
    StoredTombstone {
        tombstone: Tombstone {
            id: RemoteId(id.into()),
            version: RemoteVersion("v9".into()),
            deleted_at: UnixSeconds(100),
        },
        origin: TombstoneOrigin::Local,
        ack,
    }
}

fn conflict(local: &str) -> StoredConflict {
    StoredConflict {
        number: None,
        conflict: Conflict {
            item: RemoteId("r1".into()),
            base: BaseVersion::Absent,
            remote: RemoteSide::Exists(RemoteId("r1".into()), RemoteVersion("v3".into())),
        },
        local: LocalId(local.into()),
        local_hash: Some(ContentHash("h".into())),
        at: UnixSeconds(5),
    }
}

#[test]
fn every_kind_of_row_round_trips() {
    let _dir_guard = scratch("rows");
    let dir = _dir_guard.path().to_owned();
    let journal = Journal::open(&dir.join("sync/acct/files.sqlite")).expect("open");
    let anchor = StoredAnchor {
        anchor: Anchor("seq-4".into()),
        at: UnixSeconds(42),
    };
    journal
        .apply(&[
            Op::PutItem(item("a", Some("r1"), ItemState::Synced)),
            Op::PutItem(item("b", None, ItemState::PendingUpload)),
            Op::SetAnchor(anchor.clone()),
            Op::PutTombstone(tombstone("r8", Acknowledgement::Pending)),
            Op::AddConflict(conflict("b")),
        ])
        .expect("apply");
    assert_eq!(
        journal.items().expect("items"),
        vec![
            item("a", Some("r1"), ItemState::Synced),
            item("b", None, ItemState::PendingUpload)
        ]
    );
    assert_eq!(journal.anchor().expect("anchor"), Some(anchor));
    assert_eq!(
        journal.tombstones().expect("tombstones"),
        vec![tombstone("r8", Acknowledgement::Pending)]
    );
    let conflicts = journal.conflicts().expect("conflicts");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].number, Some(1));
    assert_eq!(conflicts[0].conflict, conflict("b").conflict);
    assert_eq!(conflicts[0].local_hash, Some(ContentHash("h".into())));
}

#[test]
fn put_replaces_by_local_id_and_the_other_ops_undo_their_rows() {
    let _dir_guard = scratch("ops");
    let dir = _dir_guard.path().to_owned();
    let journal = Journal::open(&dir.join("j.sqlite")).expect("open");
    journal
        .apply(&[
            Op::PutItem(item("a", None, ItemState::PendingUpload)),
            Op::SetAnchor(StoredAnchor {
                anchor: Anchor("x".into()),
                at: UnixSeconds(1),
            }),
            Op::PutTombstone(tombstone("r1", Acknowledgement::Pending)),
            Op::PutTombstone(tombstone("r2", Acknowledgement::Pending)),
            Op::AddConflict(conflict("a")),
        ])
        .expect("first");
    journal
        .apply(&[
            Op::PutItem(item("a", Some("r1"), ItemState::Synced)),
            Op::SetAnchor(StoredAnchor {
                anchor: Anchor("y".into()),
                at: UnixSeconds(2),
            }),
            Op::Acknowledge(RemoteId("r1".into())),
            Op::DropConflict(1),
        ])
        .expect("second");
    assert_eq!(
        journal.items().expect("items"),
        vec![item("a", Some("r1"), ItemState::Synced)]
    );
    assert_eq!(
        journal.anchor().expect("anchor").map(|a| a.anchor.0),
        Some("y".into())
    );
    assert!(journal.conflicts().expect("conflicts").is_empty());
    journal.apply(&[Op::CompactTombstones]).expect("compact");
    let left = journal.tombstones().expect("tombstones");
    assert_eq!(left, vec![tombstone("r2", Acknowledgement::Pending)]);
    journal
        .apply(&[Op::ClearAnchor, Op::DeleteItem(LocalId("a".into()))])
        .expect("clear");
    assert_eq!(journal.anchor().expect("anchor"), None);
    assert!(journal.items().expect("items").is_empty());
    assert_eq!(journal.writes(), 4);
}

#[test]
fn a_cut_write_leaves_none_of_its_rows_and_a_reopened_file_holds_what_committed() {
    let _dir_guard = scratch("cut");
    let dir = _dir_guard.path().to_owned();
    let path = dir.join("j.sqlite");
    let journal = Journal::open(&path).expect("open");
    journal
        .apply(&[Op::PutItem(item("a", None, ItemState::PendingUpload))])
        .expect("first");
    journal.cut_at(0);
    let refused = journal.apply(&[
        Op::PutItem(item("b", None, ItemState::PendingUpload)),
        Op::ClearAnchor,
    ]);
    assert!(matches!(refused, Err(JournalError::Cut)));
    drop(journal);
    let again = Journal::open(&path).expect("reopen");
    assert_eq!(
        again.items().expect("items"),
        vec![item("a", None, ItemState::PendingUpload)]
    );
}

#[test]
fn a_file_of_another_schema_version_is_emptied_and_made_again() {
    let dir = scratch("other-version");
    let path = dir.path().join("j.sqlite");
    let journal = Journal::open(&path).expect("open");
    journal
        .apply(&[Op::PutItem(item("a", Some("r1"), ItemState::Synced))])
        .expect("apply");
    drop(journal);
    Connection::open(&path)
        .expect("raw")
        .execute_batch("PRAGMA user_version = 0;")
        .expect("another version");
    let again = Journal::open(&path).expect("reopen");
    assert_eq!(again.items().expect("items"), Vec::new());
}

#[test]
fn a_row_this_build_cannot_read_is_corruption_not_a_panic() {
    let _dir_guard = scratch("corrupt");
    let dir = _dir_guard.path().to_owned();
    let path = dir.join("j.sqlite");
    drop(Journal::open(&path).expect("open"));
    Connection::open(&path)
        .expect("raw")
        .execute_batch(
            "INSERT INTO items VALUES ('a', NULL, 'p', 1, NULL, NULL, NULL, 'sideways');",
        )
        .expect("insert");
    let journal = Journal::open(&path).expect("reopen");
    assert!(matches!(journal.items(), Err(JournalError::Corrupt(_))));
}

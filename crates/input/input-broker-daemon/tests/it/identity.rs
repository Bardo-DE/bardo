use caller_identity_core::Uid;
use input_broker_core::identity::{ActiveUid, SeatSource};
use input_broker_daemon::identity::seat::SeatFile;

#[test]
fn the_seat_file_is_read_each_time_and_a_missing_one_means_no_one() {
    let dir = tempfile::tempdir().expect("scratch folder");
    let path = dir.path().join("seat0");
    let seat = SeatFile::at(&path);
    assert_eq!(seat.active_uid(), ActiveUid(None));
    std::fs::write(&path, "ACTIVE_UID=1000\n").expect("write");
    assert_eq!(seat.active_uid(), ActiveUid(Some(Uid::new(1000))));
    std::fs::write(&path, "ACTIVE_UID=1001\n").expect("write");
    assert_eq!(seat.active_uid(), ActiveUid(Some(Uid::new(1001))));
}

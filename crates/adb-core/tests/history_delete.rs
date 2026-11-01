use adb_core::workspace::history::History;
use adb_core::workspace::types::{OperationRecord, OperationStatus, Target};

fn record(id: &str, device: &str) -> OperationRecord {
    OperationRecord {
        id: id.into(),
        target: Target {
            device_id: device.into(),
            session_id: "session".into(),
            profile_id: 0,
        },
        title: "Disable app".into(),
        device_label: Some("TV".into()),
        kind: "package".into(),
        status: OperationStatus::Verified,
        created_ms: 1,
        updated_ms: 1,
        output: "the output".into(),
        error: None,
        recovery: None,
    }
}

#[test]
fn deleting_a_tv_removes_only_that_tvs_records() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("history.sqlite3");
    let history = History::open(&path).unwrap();
    history.put(&record("a", "android:one")).unwrap();
    history.put(&record("b", "android:one")).unwrap();
    history.put(&record("c", "android:two")).unwrap();

    assert_eq!(history.delete_device("android:one").unwrap(), 2);

    let left = history.list(0, 64).unwrap();
    assert_eq!(left.total, 1);
    assert_eq!(left.items[0].id, "c");
    assert_eq!(history.delete_device("android:one").unwrap(), 0);
    drop(history);
    assert_eq!(History::open(&path).unwrap().list(0, 64).unwrap().total, 1);
}

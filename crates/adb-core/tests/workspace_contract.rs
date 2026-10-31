use adb_core::workspace::{commands::*, history::History, types::*};
fn target(id: &str, user: u32) -> Target {
    Target {
        device_id: "device-1".into(),
        session_id: id.into(),
        profile_id: user,
    }
}
#[test]
fn stale_session_or_profile_cannot_apply_a_plan() {
    let current = target("new", 0);
    assert!(validate_target(Some(&current), &target("old", 0)).is_err());
    assert!(validate_target(Some(&current), &target("new", 10)).is_err());
    assert!(validate_target(None, &current).is_err());
    assert!(validate_target(Some(&current), &current).is_ok());
}
#[test]
fn typed_arguments_reject_injection_and_root_traversal() {
    assert!(package_arg("org.example.app").is_ok());
    assert!(package_arg("android").is_ok());
    assert!(package_arg("org.app;id").is_err());
    assert!(component_arg("org.app/.MainActivity").is_ok());
    assert!(component_arg("org.app/.Main;id").is_err());
    assert_eq!(
        remote_path("/sdcard/Download/../Music").unwrap(),
        "/sdcard/Music"
    );
    assert!(remote_path("/../../data").is_err());
    assert!(remote_path("/file\0name").is_err());
}
#[test]
fn interrupted_operation_reopens_as_unknown_without_replay() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("history.sqlite3");
    {
        let h = History::open(&db).unwrap();
        let row = OperationRecord {
            id: "op".into(),
            target: target("s", 0),
            title: "Disable app".into(),
            device_label: Some("Fixture TV".into()),
            kind: "package".into(),
            status: OperationStatus::Running,
            created_ms: 1,
            updated_ms: 1,
            output: "".into(),
            error: None,
            recovery: None,
        };
        h.put(&row).unwrap();
    }
    let h = History::open(&db).unwrap();
    assert_eq!(h.get("op").unwrap().status, OperationStatus::Unknown);
}
#[test]
fn history_pages_are_bounded_and_preserve_exact_output() {
    let dir = tempfile::tempdir().unwrap();
    let h = History::open(&dir.path().join("history.db")).unwrap();
    for i in 0..15 {
        h.put(&OperationRecord {
            id: format!("{i}"),
            target: target("s", 0),
            title: "Action".into(),
            device_label: Some("Fixture TV".into()),
            kind: "test".into(),
            status: OperationStatus::Verified,
            created_ms: i,
            updated_ms: i,
            output: "quotes ' and unicode ✓".into(),
            error: None,
            recovery: None,
        })
        .unwrap();
    }
    let p = h.list(1, 6).unwrap();
    assert_eq!(p.total, 15);
    assert_eq!(p.items.len(), 6);
    assert_eq!(p.items[0].output, "quotes ' and unicode ✓");
    assert_eq!(p.items[0].device_label.as_deref(), Some("Fixture TV"));
    assert!(h.list(0, 10000).is_err());
}

#[test]
fn device_identity_is_not_the_ip_address() {
    use adb_core::workspace::driver::device_identity;
    assert_eq!(
        device_identity("serial1", "vendor", "session1"),
        device_identity("serial1", "vendor", "session2")
    );
    assert_ne!(
        device_identity("serial1", "vendor", "s"),
        device_identity("serial2", "vendor", "s")
    );
    assert_ne!(
        device_identity("", "vendor", "session1"),
        device_identity("", "vendor", "session2")
    );
}

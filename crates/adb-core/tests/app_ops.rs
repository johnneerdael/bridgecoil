use adb_core::workspace::app_ops::*;
#[test]
fn does_not_confuse_uid_override_with_package_mode() {
    let s = parse(
        "Uid mode: SYSTEM_ALERT_WINDOW: ignore\nSYSTEM_ALERT_WINDOW: allow; time=-3s\n",
        "SYSTEM_ALERT_WINDOW",
    )
    .unwrap();
    assert_eq!(s.package_mode, "allow");
    assert_eq!(s.uid_mode.as_deref(), Some("ignore"));
}
#[test]
fn absent_override_means_default_while_unreadable_is_an_error() {
    assert_eq!(
        parse("No operations.\nDefault mode: allow\n", "RUN_IN_BACKGROUND")
            .unwrap()
            .package_mode,
        "default"
    );
    assert!(parse("Permission denied", "SYSTEM_ALERT_WINDOW").is_err());
    assert!(command(10, "org.app", "BAD;id", "allow").is_err());
    assert_eq!(
        command(10, "org.app", "GET_USAGE_STATS", "ignore").unwrap(),
        "cmd appops set --user 10 'org.app' GET_USAGE_STATS ignore"
    );
}

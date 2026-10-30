use adb_core::*;
#[test]
fn endpoint_requires_a_real_port() {
    for v in [
        "",
        "192.168.1.42",
        "127.0.0.1:0",
        "tv:65536",
        "http://tv:5555",
        "tv:22;id",
    ] {
        assert!(validate_endpoint(v).is_err(), "{v}");
    }
}
#[test]
fn endpoint_accepts_bracketed_ipv6_and_dns() {
    assert_eq!(validate_endpoint(" [::1]:5555 ").unwrap(), "[::1]:5555");
    assert_eq!(
        validate_endpoint("tv.local:40215").unwrap(),
        "tv.local:40215"
    );
}
#[test]
fn pairing_code_is_six_ascii_digits() {
    assert!(validate_pairing_code("012345").is_ok());
    for v in ["", "12345", "1234567", "１２３４５６", "123 56"] {
        assert!(validate_pairing_code(v).is_err());
    }
}
#[test]
fn shell_arguments_cannot_inject_commands() {
    assert_eq!(shell_quote("a'b;$(id)"), "'a'\"'\"'b;$(id)'");
}
#[test]
fn install_requires_exact_success_and_exit() {
    assert!(install_succeeded(Some(0), "Success\n"));
    for (code, text) in [
        (Some(1), "Success"),
        (None, "Success"),
        (Some(0), "Failure [INSTALL_FAILED_INVALID_APK]"),
        (Some(0), "Not Success"),
        (Some(0), "Success\nFailure"),
    ] {
        assert!(!install_succeeded(code, text));
    }
}

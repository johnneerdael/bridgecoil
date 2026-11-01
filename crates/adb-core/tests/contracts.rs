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

#[test]
fn a_play_protect_block_says_how_to_continue() {
    let message = adb_core::backend::install_rejection(
        "Failure [INSTALL_FAILED_VERIFICATION_FAILURE: Install not allowed for file:///data/app/x.tmp]",
    );
    assert!(message.contains("Play Protect"));
    assert!(message.contains("Install anyway"));
    assert!(!message.contains("INSTALL_FAILED"));
    let other = adb_core::backend::install_rejection("Failure [INSTALL_FAILED_OLDER_SDK]");
    assert!(other.contains("INSTALL_FAILED_OLDER_SDK"));
}

#[test]
fn an_app_already_on_the_tv_says_why_it_blocks_the_install() {
    let signed = adb_core::backend::install_rejection(
        "Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE: Package a.b signatures do not match previously installed version; ignoring!]",
    );
    assert!(signed.contains("signed by someone else"), "{signed}");
    assert!(signed.contains("removes its data"), "{signed}");
    let older = adb_core::backend::install_rejection(
        "Failure [INSTALL_FAILED_VERSION_DOWNGRADE: Downgrade detected: Update version code 1 is older than current 2]",
    );
    assert!(older.contains("newer version"), "{older}");
    assert!(older.contains("removes its data"), "{older}");
}

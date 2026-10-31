use adb_core::connection::connection_failure;
use libadb::{Error, error::AuthError};
#[test]
fn rejected_tls_key_explains_app_specific_pairing() {
    let error = connection_failure(Error::Auth(AuthError::TlsKeyNotTrusted));
    assert_eq!(error.kind, "pairingRequired");
    assert!(error.message.contains("Pair TV"));
    assert!(error.details.is_some());
}
#[test]
fn closed_tls_connection_does_not_claim_the_key_was_definitely_rejected() {
    let error = connection_failure(Error::Auth(AuthError::TlsClosedBeforeConnect));
    assert_eq!(error.kind, "connectionClosed");
    assert!(error.message.contains("connection port"));
}

use crate::backend::PocError;
use libadb::{
    auth::Authenticator,
    error::AuthError,
    keys::{AdbKey, KeyError},
    transport::tls::TlsError,
};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStage {
    Preparing,
    Connecting,
    Negotiating,
    WaitingForApproval,
    Inspecting,
    Pairing,
}
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionProgress {
    pub stage: ConnectionStage,
}
pub(crate) struct ConnectionAuth<F> {
    key: AdbKey,
    notify: Arc<F>,
    pub offered: Arc<AtomicBool>,
    allow_legacy: bool,
}
impl<F: Fn(ConnectionProgress)> ConnectionAuth<F> {
    pub fn new(key: AdbKey, allow_legacy: bool, notify: Arc<F>) -> Self {
        Self {
            key,
            notify,
            offered: Arc::new(AtomicBool::new(false)),
            allow_legacy,
        }
    }
}
impl<F: Fn(ConnectionProgress)> Authenticator for ConnectionAuth<F> {
    type Error = KeyError;
    async fn sign(&mut self, token: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Authenticator::sign(&mut self.key, token).await
    }
    fn public_key(&self) -> &[u8] {
        if !self.offered.swap(true, Ordering::SeqCst) {
            (self.notify)(ConnectionProgress {
                stage: ConnectionStage::WaitingForApproval,
            });
        }
        Authenticator::public_key(&self.key)
    }
    fn offer_public_key(&self) -> bool {
        self.allow_legacy
    }
}
pub fn connection_failure(error: libadb::Error<TlsError<std::io::Error>>) -> PocError {
    let details = error.to_string();
    let (kind, message) = match &error {
        libadb::Error::Auth(AuthError::TlsKeyNotTrusted) => (
            "pairingRequired",
            "The TV rejected this app's key. Use Pair TV for this installation, then connect using the separate connection port. Reinstalling an app creates a new key.",
        ),
        libadb::Error::Auth(AuthError::TlsClosedBeforeConnect) => (
            "connectionClosed",
            "The TV closed the secure connection before ADB was ready. Check the separate connection port and pair this installation if it has not been paired.",
        ),
        libadb::Error::Auth(AuthError::KeyNotTrusted) => (
            "authorizationRequired",
            "The TV has not authorized this app's key. Use Pair TV for Wireless debugging, or enable legacy ADB and approve this app's key on the TV.",
        ),
        libadb::Error::Io(TlsError::HandshakeClosed) => (
            "tls",
            "The TV closed the TLS handshake. Check that this is the connection port from the main Wireless debugging screen, rather than the pairing port.",
        ),
        libadb::Error::Io(TlsError::Tls(_)) => (
            "tls",
            "The ADB TLS handshake failed. This app already accepts self-signed ADB certificates. Check the connection port and read the technical details below.",
        ),
        libadb::Error::Io(TlsError::Io(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
            (
                "connectionRefused",
                "No ADB service answered on this port. Wireless debugging's connection port can change; check the TV's current address.",
            )
        }
        _ => (
            "connection",
            "The ADB connection did not become ready. Check the TV's connection address and authorization, then read the technical details below.",
        ),
    };
    PocError {
        kind: kind.into(),
        message: message.into(),
        details: Some(details),
    }
}

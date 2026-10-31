//! Bounded APK signature-integrity inspection. A valid signature does not establish publisher trust.
use apk_signing::{
    archive::ApkLayout,
    sign::{Schemes, VerifyError},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
const MAX_BLOCK: usize = 8 * 1024 * 1024;
const MAX_JAR_TOTAL: u64 = 256 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SignatureState {
    Verified,
    Unsupported,
    Unsigned,
    Invalid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signer {
    pub sha256: String,
    pub subject: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningInfo {
    pub state: SignatureState,
    pub schemes: Vec<String>,
    pub signers: Vec<Signer>,
    pub reason: Option<String>,
}
impl SigningInfo {
    fn result(state: SignatureState, reason: impl Into<String>) -> Self {
        Self {
            state,
            schemes: Vec::new(),
            signers: Vec::new(),
            reason: Some(reason.into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningLimitError {
    Malformed,
    Limit(&'static str),
}
type LimitResult<T> = Result<T, SigningLimitError>;
fn lp<'a>(input: &mut &'a [u8]) -> LimitResult<&'a [u8]> {
    let length = input.get(..4).ok_or(SigningLimitError::Malformed)?;
    let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
    let end = length.checked_add(4).ok_or(SigningLimitError::Malformed)?;
    let value = input.get(4..end).ok_or(SigningLimitError::Malformed)?;
    *input = &input[end..];
    Ok(value)
}
fn skip(input: &mut &[u8], count: usize) -> LimitResult<()> {
    *input = input.get(count..).ok_or(SigningLimitError::Malformed)?;
    Ok(())
}
fn sequence(
    mut input: &[u8],
    limit: usize,
    mut check: impl FnMut(&[u8]) -> LimitResult<()>,
) -> LimitResult<()> {
    let mut count = 0;
    while !input.is_empty() {
        count += 1;
        if count > limit {
            return Err(SigningLimitError::Limit("too many signing records"));
        }
        check(lp(&mut input)?)?;
    }
    Ok(())
}
fn length_bound(value: &[u8], max: usize) -> LimitResult<()> {
    if value.len() > max {
        Err(SigningLimitError::Limit("oversized signing metadata"))
    } else {
        Ok(())
    }
}
fn algorithm_record(mut value: &[u8], max: usize) -> LimitResult<()> {
    skip(&mut value, 4)?;
    length_bound(lp(&mut value)?, max)?;
    if !value.is_empty() {
        return Err(SigningLimitError::Malformed);
    }
    Ok(())
}
fn check_signers(mut value: &[u8], v3: bool) -> LimitResult<()> {
    let signers = lp(&mut value)?;
    if !value.is_empty() {
        return Err(SigningLimitError::Malformed);
    }
    sequence(signers, 16, |mut signer| {
        let mut signed = lp(&mut signer)?;
        if v3 {
            skip(&mut signer, 8)?;
        }
        let signatures = lp(&mut signer)?;
        let public_key = lp(&mut signer)?;
        if !signer.is_empty() {
            return Err(SigningLimitError::Malformed);
        }
        length_bound(public_key, 2048)?;
        sequence(signatures, 64, |record| algorithm_record(record, 1024))?;
        let digests = lp(&mut signed)?;
        let certificates = lp(&mut signed)?;
        if v3 {
            skip(&mut signed, 8)?;
        }
        let attributes = lp(&mut signed)?;
        // apksigner appends an empty field. The verifier authenticates the original bytes,
        // then reads the known fields; bound any signed extension without rejecting it.
        length_bound(signed, 65536)?;
        sequence(digests, 64, |record| algorithm_record(record, 128))?;
        sequence(certificates, 16, |certificate| {
            length_bound(certificate, 65536)
        })?;
        sequence(attributes, 64, |attribute| {
            if attribute.len() < 4 {
                return Err(SigningLimitError::Malformed);
            }
            length_bound(attribute, 65536)
        })
    })
}
/// Validate allocation-driving records before passing bytes to the pinned verifier.
pub fn validate_block_limits(bytes: &[u8]) -> LimitResult<()> {
    if bytes.len() > MAX_BLOCK {
        return Err(SigningLimitError::Limit("signing block exceeds 8 MiB"));
    }
    if bytes.len() < 32 || !bytes.ends_with(b"APK Sig Block 42") {
        return Err(SigningLimitError::Malformed);
    }
    let size = (bytes.len() - 8) as u64;
    if u64::from_le_bytes(bytes[..8].try_into().unwrap()) != size
        || u64::from_le_bytes(
            bytes[bytes.len() - 24..bytes.len() - 16]
                .try_into()
                .unwrap(),
        ) != size
    {
        return Err(SigningLimitError::Malformed);
    }
    let mut body = &bytes[8..bytes.len() - 24];
    let mut count = 0;
    while !body.is_empty() {
        count += 1;
        if count > 64 {
            return Err(SigningLimitError::Limit("too many signing-block entries"));
        }
        let size = u64::from_le_bytes(
            body.get(..8)
                .ok_or(SigningLimitError::Malformed)?
                .try_into()
                .unwrap(),
        );
        let size = usize::try_from(size).map_err(|_| SigningLimitError::Malformed)?;
        let end = size
            .checked_add(8)
            .filter(|end| size >= 4 && *end <= body.len())
            .ok_or(SigningLimitError::Malformed)?;
        let id = u32::from_le_bytes(body[8..12].try_into().unwrap());
        let data = &body[12..end];
        match id {
            0x7109_871a => check_signers(data, false)?,
            0xf053_68c0 => check_signers(data, true)?,
            _ => {}
        }
        body = &body[end..];
    }
    Ok(())
}
pub fn inspect(path: &Path) -> Result<SigningInfo, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    if length == 0 || length > crate::apk::MAX_APK_BYTES {
        return Ok(SigningInfo::result(
            SignatureState::Invalid,
            "APK file size is outside the supported bounds.",
        ));
    }
    let layout = match ApkLayout::read(&mut file) {
        Ok(layout) => layout,
        Err(e) => return Ok(SigningInfo::result(SignatureState::Invalid, e.to_string())),
    };
    if let Some(range) = layout.signing_block {
        let length = range
            .end
            .checked_sub(range.start)
            .ok_or("Invalid signing block range")?;
        if length > MAX_BLOCK as u64 {
            return Ok(SigningInfo::result(
                SignatureState::Unsupported,
                "Signing metadata exceeds local verification limits. Android will verify during installation.",
            ));
        }
        file.seek(SeekFrom::Start(range.start))
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![0; length as usize];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        if let Err(e) = validate_block_limits(&bytes) {
            return Ok(match e {
                SigningLimitError::Malformed => {
                    SigningInfo::result(SignatureState::Invalid, "Malformed APK signing records.")
                }
                SigningLimitError::Limit(reason) => {
                    SigningInfo::result(SignatureState::Unsupported, reason)
                }
            });
        }
    }
    let mut archive = match zip::ZipArchive::new(File::open(path).map_err(|e| e.to_string())?) {
        Ok(archive) => archive,
        Err(e) => return Ok(SigningInfo::result(SignatureState::Invalid, e.to_string())),
    };
    if archive.len() > 65534 {
        return Ok(SigningInfo::result(
            SignatureState::Unsupported,
            "Too many ZIP entries for bounded local verification.",
        ));
    }
    let mut v1 = false;
    let mut metadata_entries = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.name().to_ascii_uppercase();
        let jar_signature = name.starts_with("META-INF/")
            && [".RSA", ".DSA", ".EC"]
                .iter()
                .any(|extension| name.ends_with(extension));
        v1 |= jar_signature;
        let metadata = name == "ANDROIDMANIFEST.XML"
            || name.starts_with("META-INF/")
                && [".MF", ".SF", ".RSA", ".DSA", ".EC"]
                    .iter()
                    .any(|extension| name.ends_with(extension));
        if metadata {
            metadata_entries += 1;
            if metadata_entries > 64 {
                return Ok(SigningInfo::result(
                    SignatureState::Unsupported,
                    "Too many signature metadata entries.",
                ));
            }
            let limit = if name == "ANDROIDMANIFEST.XML" {
                2 * 1024 * 1024
            } else if name.ends_with(".MF") || name.ends_with(".SF") {
                8 * 1024 * 1024
            } else {
                1024 * 1024
            };
            let mut data = Vec::new();
            entry
                .by_ref()
                .take(limit + 1)
                .read_to_end(&mut data)
                .map_err(|e| e.to_string())?;
            if data.len() as u64 > limit {
                return Ok(SigningInfo::result(
                    SignatureState::Unsupported,
                    "Signature metadata exceeds bounded local verification limits.",
                ));
            }
            if name == "ANDROIDMANIFEST.XML"
                && crate::manifest::validate_verifier_pool(&data).is_err()
            {
                return Ok(SigningInfo::result(
                    SignatureState::Unsupported,
                    "Manifest expansion exceeds bounded local verification limits. Android will validate it during installation.",
                ));
            }
        }
    }
    if v1 {
        // JAR signing hashes uncompressed entries; bound actual work even when ZIP sizes lie.
        let mut total = 0u64;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let limit = (128 * 1024 * 1024).min(MAX_JAR_TOTAL.saturating_sub(total));
            let bytes = std::io::copy(&mut entry.by_ref().take(limit + 1), &mut std::io::sink())
                .map_err(|e| e.to_string())?;
            if bytes > limit {
                return Ok(SigningInfo::result(
                    SignatureState::Unsupported,
                    "Legacy JAR verification exceeds the bounded work budget. Android will verify this APK during installation.",
                ));
            }
            total += bytes;
        }
    }
    let verification = match apk_signing::sign::verify(path) {
        Ok(value) => value,
        Err(apk_signing::Error::Verify(VerifyError::Unsigned)) => {
            return Ok(SigningInfo::result(
                SignatureState::Unsigned,
                "The APK has no supported signature.",
            ));
        }
        Err(apk_signing::Error::Verify(
            e @ (VerifyError::KeyRotationUnsupported
            | VerifyError::NoSupportedAlgorithm { .. }
            | VerifyError::Coverage(_)),
        )) => {
            return Ok(SigningInfo::result(
                SignatureState::Unsupported,
                format!(
                    "Local verification is unavailable: {e}. Android will verify during installation."
                ),
            ));
        }
        Err(e) => return Ok(SigningInfo::result(SignatureState::Invalid, e.to_string())),
    };
    if verification.signers().is_empty() || verification.signers().len() > 16 {
        return Ok(SigningInfo::result(
            SignatureState::Unsupported,
            "Unexpected signer count.",
        ));
    }
    let schemes = [
        ("v1", Schemes::V1),
        ("v2", Schemes::V2),
        ("v3", Schemes::V3),
    ]
    .into_iter()
    .filter(|(_, scheme)| verification.schemes.contains(*scheme))
    .map(|(name, _)| name.into())
    .collect();
    let signers = verification
        .signers()
        .iter()
        .map(|signer| Signer {
            sha256: hex::encode(signer.certificate().sha256_fingerprint()),
            subject: signer.certificate().subject().chars().take(512).collect(),
        })
        .collect();
    Ok(SigningInfo {
        state: SignatureState::Verified,
        schemes,
        signers,
        reason: None,
    })
}

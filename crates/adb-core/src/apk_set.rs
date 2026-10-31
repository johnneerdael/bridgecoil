use crate::{
    apk::{self, ApkInfo},
    signing::{self, SignatureState, SigningInfo},
};
use serde::Serialize;
use std::{collections::BTreeSet, path::Path};
#[derive(Debug, Clone, Serialize)]
pub struct SignedApkInfo {
    #[serde(flatten)]
    pub info: ApkInfo,
    pub signing: SigningInfo,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApkSetInfo {
    pub package: String,
    pub version_code: u64,
    pub bytes: u64,
    pub files: Vec<SignedApkInfo>,
    pub warnings: Vec<String>,
}
pub fn inspect_part(path: &Path) -> Result<SignedApkInfo, String> {
    let info = apk::inspect_package(path)?;
    let signing = signing::inspect(path)?;
    match signing.state {
        SignatureState::Unsigned => {
            return Err("This APK is unsigned. Android requires a signed package.".into());
        }
        SignatureState::Invalid => {
            return Err(format!(
                "APK signature or contents are invalid: {}",
                signing.reason.as_deref().unwrap_or("verification failed")
            ));
        }
        _ => {}
    }
    Ok(SignedApkInfo { info, signing })
}
pub fn assemble_set(
    mut files: Vec<SignedApkInfo>,
    sdk: u32,
    abis: &[String],
) -> Result<ApkSetInfo, String> {
    if files.is_empty() || files.len() > 64 {
        return Err("Choose one APK, or a base APK and up to 63 matching splits.".into());
    }
    let bases: Vec<_> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.info.split.is_none())
        .map(|(i, _)| i)
        .collect();
    if bases.len() != 1 {
        return Err("Choose exactly one base APK together with its matching split APKs.".into());
    }
    files.swap(0, bases[0]);
    let package = files[0].info.package.clone();
    let version_code = files[0].info.version_code;
    if files[0].info.requires_splits && files.len() == 1 {
        return Err("This base APK requires split APKs. Choose its matching configuration and feature files too.".into());
    }
    let mut names = BTreeSet::new();
    let mut types = BTreeSet::new();
    let mut bytes = 0u64;
    let mut expected_signers = None;
    let mut warnings = Vec::new();
    for file in &files {
        let info = &file.info;
        if info.package != package || info.version_code != version_code {
            return Err("All APKs must belong to the same package and version.".into());
        }
        if let Some(name) = &info.split {
            if !apk::valid_split_name(name) || !names.insert(name.clone()) {
                return Err("The selected set contains an invalid or duplicate split name.".into());
            }
        }
        types.extend(info.split_types.iter().cloned());
        bytes = bytes
            .checked_add(info.bytes)
            .filter(|v| *v <= apk::MAX_APK_BYTES)
            .ok_or("The complete APK set must be no larger than 1 GiB.")?;
        if info.min_sdk.is_some_and(|min| sdk != 0 && min > sdk) {
            return Err("One of these APKs requires a newer Android version than the TV.".into());
        }
        if !info.abis.is_empty() && !abis.is_empty() && !info.abis.iter().any(|a| abis.contains(a))
        {
            return Err(
                "One of these APKs contains native libraries for a different TV architecture."
                    .into(),
            );
        }
        match file.signing.state{
   SignatureState::Verified=>{let signers:BTreeSet<_>=file.signing.signers.iter().map(|s|s.sha256.to_lowercase()).collect();if signers.is_empty(){return Err("No signing certificate was verified.".into());}if expected_signers.as_ref().is_some_and(|old|old!=&signers){return Err("The selected APKs are signed by different certificates.".into());}expected_signers=Some(signers);}
   SignatureState::Unsupported=>warnings.push("Some signatures cannot be verified locally. Android will check the complete set during installation.".into()),
   SignatureState::Invalid|SignatureState::Unsigned=>return Err("The selected APKs contain an invalid or unsigned package.".into()),
  }
    }
    for file in &files {
        if file
            .info
            .required_splits
            .iter()
            .any(|name| !names.contains(name))
        {
            return Err("A required feature split is missing from the selection.".into());
        }
        if file
            .info
            .config_for_split
            .as_ref()
            .is_some_and(|name| !names.contains(name))
        {
            return Err("A configuration split's feature APK is missing.".into());
        }
        if file
            .info
            .required_split_types
            .iter()
            .any(|kind| !types.contains(kind))
        {
            return Err("A required configuration type is missing from the selection.".into());
        }
    }
    warnings.sort();
    warnings.dedup();
    Ok(ApkSetInfo {
        package,
        version_code,
        bytes,
        files,
        warnings,
    })
}

use super::release::Asset;
use crate::backend::PocError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolId {
    ShizukuNext,
    Porter,
    PorterCompat,
    Termux,
    TermuxBoot,
    Backdrop,
    Projectivy,
    Monet,
    At4k,
}

/// The launchers Bridgecoil offers, in the order it lists them.
pub const LAUNCHERS: [ToolId; 4] = [
    ToolId::Backdrop,
    ToolId::Projectivy,
    ToolId::Monet,
    ToolId::At4k,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolSpec {
    pub id: ToolId,
    pub name: &'static str,
    pub repo: &'static str,
    pub package: &'static str,
    /// Releases signed by anyone else are refused, even from the right repository.
    pub signer_sha256: &'static str,
}

pub const SHIZUKU_PACKAGE: &str = "moe.shizuku.privileged.api";
const NEXT_SIGNER: &str = "06360bb800d1a117c3a22c91a58e638bcd37281cb4d644fcfac5d7238c654293";
const PORTER_SIGNER: &str = "c6302c2b163c7f42f6d8bd658e53387880511020136a083f0a6a4bdc20b4bfa0";
const TERMUX_SIGNER: &str = "b6da01480eefd5fbf2cd3771b8d1021ec791304bdd6c4bf41d3faabad48ee5e1";
const BACKDROP_SIGNER: &str = "18a076c8f5666fb103184caf1a98c753a044cc54ffc8ddc54cdd13ccc339f93d";
const PROJECTIVY_SIGNER: &str = "f6697bf4082ee97511e4de07863193884a015b7ab5860430321bda1042b0aadd";
const MONET_SIGNER: &str = "4bec386a6baae56f0694d3be54f67e0625b973109e0dbb947df2a5d0254975e0";
const AT4K_SIGNER: &str = "bfc84ab1296c72653ebca20a2f4cd293bc342a67dd4c61df931edfbad5271685";

pub fn spec(id: ToolId) -> ToolSpec {
    let (name, repo, package, signer_sha256) = match id {
        ToolId::ShizukuNext => (
            "Shizuku-Next",
            "rushiranpise/Shizuku-Next",
            SHIZUKU_PACKAGE,
            NEXT_SIGNER,
        ),
        ToolId::Porter => (
            "Porter",
            "d4rken-org/porter",
            "eu.darken.porter",
            PORTER_SIGNER,
        ),
        ToolId::PorterCompat => (
            "Porter Compatibility",
            "d4rken-org/porter",
            SHIZUKU_PACKAGE,
            PORTER_SIGNER,
        ),
        ToolId::Termux => ("Termux", "termux/termux-app", "com.termux", TERMUX_SIGNER),
        ToolId::TermuxBoot => (
            "Termux:Boot",
            "termux/termux-boot",
            "com.termux.boot",
            TERMUX_SIGNER,
        ),
        ToolId::Backdrop => (
            "Backdrop",
            "johnneerdael/backdrop-launcher",
            "nl.neerdael.backdrop_launcher",
            BACKDROP_SIGNER,
        ),
        ToolId::Projectivy => (
            "Projectivy",
            "spocky/miproja1",
            "com.spocky.projengmenu",
            PROJECTIVY_SIGNER,
        ),
        ToolId::Monet => (
            "Monet",
            "Klevico/Monet-Launcher",
            "com.klevico.monet",
            MONET_SIGNER,
        ),
        ToolId::At4k => ("AT4K", "avadhesh18/at4k", "com.overdevs.at4k", AT4K_SIGNER),
    };
    ToolSpec {
        id,
        name,
        repo,
        package,
        signer_sha256,
    }
}

fn matches(id: ToolId, name: &str) -> bool {
    match id {
        ToolId::ShizukuNext => name.starts_with("shizuku-v") && name.ends_with("-next.apk"),
        ToolId::Porter => name.starts_with("porter-v") && name.ends_with("-release.apk"),
        ToolId::PorterCompat => {
            name.starts_with("porter-compat-v") && name.ends_with("-release.apk")
        }
        ToolId::Termux => name.starts_with("termux-app_v") && name.ends_with(".apk"),
        ToolId::TermuxBoot => name.starts_with("termux-boot-app_v") && name.ends_with(".apk"),
        // Backdrop also publishes a build per ABI. The one with no ABI in its name has them all.
        ToolId::Backdrop => name
            .strip_prefix("backdrop-launcher-")
            .and_then(|rest| rest.strip_suffix(".apk"))
            .is_some_and(|version| !version.contains('-')),
        ToolId::Projectivy => {
            name.starts_with("ProjectivyLauncher-") && name.ends_with("-release.apk")
        }
        ToolId::Monet => name == "monet-launcher.apk",
        ToolId::At4k => name == "app-release.apk",
    }
}

/// Termux publishes one build per ABI. The rest publish a single APK with every ABI inside.
pub fn select_asset<'a>(
    id: ToolId,
    assets: &'a [Asset],
    abis: &[String],
) -> Result<&'a Asset, PocError> {
    let candidates: Vec<_> = assets.iter().filter(|a| matches(id, &a.name)).collect();
    let ending = |suffix: &str| {
        let wanted = format!("_{suffix}.apk");
        candidates
            .iter()
            .find(|a| a.name.ends_with(&wanted))
            .copied()
    };
    let picked = if id == ToolId::Termux {
        abis.iter()
            .find_map(|abi| ending(abi))
            .or_else(|| ending("universal"))
    } else {
        candidates.first().copied()
    };
    picked.ok_or_else(|| {
        PocError::new(
            "toolUnavailable",
            format!(
                "The latest {} release has no build for this TV.",
                spec(id).name
            ),
        )
    })
}

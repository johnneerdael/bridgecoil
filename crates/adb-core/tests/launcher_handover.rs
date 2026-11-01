//! A TV's own launcher can outrank the one chosen, as Android TV's does at priority 2. Android
//! then ignores the choice, so turning the stock launcher off is the only way to hand over.
use adb_core::{
    backend::PocError,
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, PackageAction, Workspace},
        history::History,
        queries::{Query, query},
        types::Target,
    },
};
use adb_core::{
    toolkit::catalog::{ToolId, spec},
    workspace::metadata::Metadata,
};
use async_trait::async_trait;
use serde_json::json;

const STATE: &str = "versionCode=3 minSdk=24\n User 0: installed=true hidden=false suspended=false stopped=false enabled=1\n";
const STOCK: &str = "com.vendor.home";

struct Tv {
    role: Result<&'static str, ()>,
    homes: &'static str,
}
fn tv(role: &'static str) -> Tv {
    Tv {
        role: Ok(role),
        homes: "com.vendor.home/.Main\norg.fresh/.Home\ncom.android.tv.settings/.FallbackHome\n",
    }
}
#[async_trait]
impl ShellDriver for Tv {
    fn current_target(&self) -> Option<Target> {
        Some(target())
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        let mut exit_code = Some(0);
        let stdout = if c.starts_with("dumpsys package") {
            STATE.into()
        } else if c.contains("resolve-activity") {
            "priority=2 preferredOrder=0\ncom.vendor.home/.Main\n".into()
        } else if c.contains("query-activities") {
            self.homes.into()
        } else if c.contains("get-role-holders") {
            match self.role {
                Ok(role) => format!("{role}\n"),
                Err(()) => {
                    exit_code = Some(255);
                    String::new()
                }
            }
        } else if c.contains("pm list packages") {
            "package:com.vendor.home\npackage:com.spocky.projengmenu\npackage:com.klevico.monet\n"
                .into()
        } else {
            String::new()
        };
        Ok(ShellOutput {
            stdout,
            stderr: String::new(),
            exit_code,
        })
    }
    async fn metadata(
        &mut self,
        _: &Target,
        packages: &[String],
    ) -> Result<Option<Vec<Metadata>>, PocError> {
        let signed = |package: &str, signer: &str, version: &str| Metadata {
            package: package.into(),
            version_name: Some(version.into()),
            signers: vec![signer.into()],
            ..Default::default()
        };
        assert_eq!(packages, ["com.spocky.projengmenu", "com.klevico.monet"]);
        Ok(Some(vec![
            signed(
                "com.spocky.projengmenu",
                &spec(ToolId::Projectivy).signer_sha256.to_uppercase(),
                "4.72",
            ),
            signed("com.klevico.monet", "patched", "1.1.0"),
        ]))
    }
}
fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 0,
    }
}
fn workspace(dir: &tempfile::TempDir) -> Workspace {
    Workspace::new(History::open(&dir.path().join("ops.db")).unwrap())
}
fn stock(action: PackageAction) -> Action {
    Action::Package {
        package: STOCK.into(),
        action,
    }
}

#[tokio::test]
async fn stock_can_be_turned_off_once_another_launcher_is_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    let review = w
        .prepare(
            &mut tv("org.fresh"),
            &target(),
            stock(PackageAction::Disable),
        )
        .await
        .unwrap();
    assert!(review.reversible, "turning stock off needs a way back");
}

#[tokio::test]
async fn the_launcher_in_use_stays_protected_without_a_real_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    for mut device in [
        tv(STOCK),
        tv(""),
        tv("org.gone"),
        tv("com.android.tv.settings"),
        Tv {
            role: Err(()),
            homes: "com.vendor.home/.Main\ncom.android.tv.settings/.FallbackHome\n",
        },
    ] {
        let e = w
            .prepare(&mut device, &target(), stock(PackageAction::Disable))
            .await
            .unwrap_err();
        assert_eq!(e.kind, "protected");
    }
}

/// Android 9 has no role service, so nothing says which launcher was chosen.
#[tokio::test]
async fn without_roles_any_other_launcher_counts_as_a_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    let mut older = Tv {
        role: Err(()),
        ..tv("")
    };
    w.prepare(&mut older, &target(), stock(PackageAction::Disable))
        .await
        .unwrap();
    for action in [PackageAction::Remove, PackageAction::ClearData] {
        let e = w
            .prepare(&mut older, &target(), stock(action))
            .await
            .unwrap_err();
        assert_eq!(e.kind, "protected");
    }
}

#[tokio::test]
async fn a_replacement_only_excuses_turning_off() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    for action in [PackageAction::Remove, PackageAction::ClearData] {
        let e = w
            .prepare(&mut tv("org.fresh"), &target(), stock(action))
            .await
            .unwrap_err();
        assert_eq!(e.kind, "protected");
    }
}

#[tokio::test]
async fn the_replacement_is_checked_again_when_applying() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    let review = w
        .prepare(
            &mut tv("org.fresh"),
            &target(),
            stock(PackageAction::Disable),
        )
        .await
        .unwrap();
    let e = w.apply(&mut tv(STOCK), &review.id).await.unwrap_err();
    assert_eq!(e.kind, "protected");
}

#[tokio::test]
async fn a_choice_android_ignores_says_what_happened() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    let review = w
        .prepare(
            &mut tv(STOCK),
            &target(),
            Action::DefaultHome {
                component: "org.fresh/.Home".into(),
            },
        )
        .await
        .unwrap();
    assert!(review.title.contains("org.fresh"), "{}", review.title);
    let done = w.apply(&mut tv("org.fresh"), &review.id).await.unwrap();
    let error = done.error.unwrap();
    assert!(error.contains("Home still opens com.vendor.home"), "{error}");
}

#[tokio::test]
async fn launchers_report_which_one_was_chosen() {
    let r = query(&mut tv("org.fresh"), &target(), Query::Launchers)
        .await
        .unwrap();
    assert_eq!(r["chosen"], json!("org.fresh"));
    assert_eq!(r["roles"], json!(true));
    assert_eq!(r["current"], json!("com.vendor.home/.Main"));

    let older = Tv {
        role: Err(()),
        ..tv("")
    };
    let r = query(&mut { older }, &target(), Query::Launchers)
        .await
        .unwrap();
    assert_eq!(r["chosen"], json!(null));
    assert_eq!(r["roles"], json!(false));
}

/// What the app needs to offer Install, Update or nothing for each launcher.
#[tokio::test]
async fn lists_each_launcher_with_what_the_tv_has() {
    let mut d = tv("com.vendor.home\n");
    let found = query(&mut d, &target(), Query::Launchers).await.unwrap();
    let row = |package: &str| {
        found["catalog"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["package"] == package)
            .unwrap()
            .clone()
    };
    assert_eq!(found["catalog"].as_array().unwrap().len(), 4);
    let projectivy = row("com.spocky.projengmenu");
    assert_eq!(projectivy["tool"], "projectivy");
    assert_eq!(projectivy["version"], "4.72");
    assert_eq!(projectivy["official"], true);
    assert_eq!(row("com.klevico.monet")["official"], false);
    let absent = row("com.overdevs.at4k");
    assert_eq!(absent["installed"], false);
    assert!(absent["official"].is_null());
}

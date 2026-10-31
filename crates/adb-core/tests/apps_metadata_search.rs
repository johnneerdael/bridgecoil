use adb_core::{
    backend::PocError,
    workspace::{
        driver::{ShellDriver, ShellOutput},
        metadata::Metadata,
        queries::{Query, query},
        types::Target,
    },
};
use async_trait::async_trait;
struct Fake {
    target: Target,
    fail_metadata: bool,
}
#[async_trait]
impl ShellDriver for Fake {
    fn current_target(&self) -> Option<Target> {
        Some(self.target.clone())
    }
    async fn shell(&mut self, _: &Target, _: &str) -> Result<ShellOutput, PocError> {
        Ok(ShellOutput {
            stdout: "package:org.alpha\npackage:org.beta\n".into(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
    async fn labels(
        &mut self,
        _: &Target,
        _: &[String],
    ) -> Result<Option<Vec<Metadata>>, PocError> {
        if self.fail_metadata {
            return Err(PocError::new(
                "metadata",
                "OEM denied PackageManager inspection",
            ));
        }
        Ok(Some(vec![
            Metadata {
                package: "org.alpha".into(),
                label: Some("Zebra".into()),
                ..Default::default()
            },
            Metadata {
                package: "org.beta".into(),
                label: Some("Player".into()),
                ..Default::default()
            },
        ]))
    }
}
fn driver(fail_metadata: bool) -> Fake {
    Fake {
        target: Target {
            device_id: "d".into(),
            session_id: "s".into(),
            profile_id: 10,
        },
        fail_metadata,
    }
}
#[tokio::test]
async fn search_finds_a_display_name_and_keeps_the_package_target() {
    let mut d = driver(false);
    let t = d.target.clone();
    let page = query(
        &mut d,
        &t,
        Query::Packages {
            filter: "all".into(),
            search: "player".into(),
            page: 0,
            page_size: 4,
        },
    )
    .await
    .unwrap();
    assert_eq!(page["total"], 1);
    assert_eq!(page["items"][0]["package"], "org.beta");
    assert_eq!(page["items"][0]["label"], "Player");
}
#[tokio::test]
async fn oem_metadata_failure_preserves_browsable_package_ids() {
    let mut d = driver(true);
    let t = d.target.clone();
    let page = query(
        &mut d,
        &t,
        Query::Packages {
            filter: "all".into(),
            search: "".into(),
            page: 0,
            page_size: 4,
        },
    )
    .await
    .unwrap();
    assert_eq!(page["total"], 2);
    assert_eq!(page["items"][0]["package"], "org.alpha");
    assert!(page["items"][0]["label"].is_null());
    assert!(page["metadataNote"].is_string());
}

struct DetailsDriver {
    target: Target,
}
#[async_trait]
impl ShellDriver for DetailsDriver {
    fn current_target(&self) -> Option<Target> {
        Some(self.target.clone())
    }
    async fn shell(&mut self, _: &Target, command: &str) -> Result<ShellOutput, PocError> {
        let stdout = if command.contains("LEANBACK_LAUNCHER") {
            "org.demo/.TvZ"
        } else if command.contains("category.LAUNCHER") {
            "org.demo/.PhoneA"
        } else {
            "versionCode=1\nUser 10: installed=true enabled=0 stopped=false"
        };
        Ok(ShellOutput {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}
#[tokio::test]
async fn normal_open_prefers_the_discovered_tv_activity_over_alphabetical_order() {
    let t = driver(false).target;
    let mut d = DetailsDriver { target: t.clone() };
    let detail = query(
        &mut d,
        &t,
        Query::Package {
            package: "org.demo".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(detail["launchActivity"], "org.demo/.TvZ");
}

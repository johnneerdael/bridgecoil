use adb_core::{
    backend::PocError,
    ratings::{
        Level, Rating, RatingList,
        groups::{Group, group_for},
        parse_list,
    },
    workspace::{
        driver::{ShellDriver, ShellOutput},
        metadata::Metadata,
        queries::{Query, query_rated},
        types::Target,
    },
};
use async_trait::async_trait;
use serde_json::{Value, json};

struct Fake {
    all: Vec<&'static str>,
    user: Vec<&'static str>,
    labels: Vec<(&'static str, &'static str)>,
    fail_user: bool,
    commands: Vec<String>,
}
fn packages(names: &[&str]) -> String {
    names.iter().map(|p| format!("package:{p}\n")).collect()
}
#[async_trait]
impl ShellDriver for Fake {
    fn current_target(&self) -> Option<Target> {
        Some(target())
    }
    async fn labels(
        &mut self,
        _: &Target,
        requested: &[String],
    ) -> Result<Option<Vec<Metadata>>, PocError> {
        Ok(Some(
            requested
                .iter()
                .map(|package| Metadata {
                    package: package.clone(),
                    label: self
                        .labels
                        .iter()
                        .find(|(p, _)| p == package)
                        .map(|(_, label)| (*label).to_owned()),
                    ..Default::default()
                })
                .collect(),
        ))
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(c.into());
        let fail = self.fail_user && c.ends_with("-3");
        let stdout = if c.ends_with("-3") {
            packages(&self.user)
        } else if c.starts_with("pm list packages") {
            packages(&self.all)
        } else if c.starts_with("dumpsys package") {
            " versionName=1\n versionCode=3\n User 0: installed=true enabled=0 stopped=false\n"
                .into()
        } else {
            String::new()
        };
        Ok(ShellOutput {
            stdout,
            stderr: String::new(),
            exit_code: Some(if fail { 1 } else { 0 }),
        })
    }
}
fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 0,
    }
}
fn ratings() -> RatingList {
    parse_list(
        r#"{
        "com.good":{"removal":"Recommended"},
        "com.good.two":{"removal":"Recommended"},
        "com.mid":{"removal":"Advanced"},
        "com.hard":{"removal":"Expert"},
        "com.risky":{"removal":"Unsafe","description":"Vital","list":"Aosp",
                     "dependencies":["com.mid","com.absent"],"neededBy":["com.hard"]},
        "com.mine.rated":{"removal":"Unsafe"},
        "com.odd":{"removal":"Unlisted"}
    }"#,
    )
    .unwrap()
}
fn fake() -> Fake {
    Fake {
        all: vec![
            "com.risky",
            "com.hard",
            "com.unknown",
            "com.odd",
            "com.mid",
            "com.good.two",
            "com.good",
            "com.mine",
            "com.mine.rated",
        ],
        user: vec!["com.mine", "com.mine.rated"],
        labels: vec![("com.good", "Zebra"), ("com.good.two", "apple")],
        fail_user: false,
        commands: vec![],
    }
}
fn packages_query(filter: &str, search: &str, page: usize, page_size: usize) -> Query {
    Query::Packages {
        filter: filter.into(),
        search: search.into(),
        page,
        page_size,
    }
}
fn ids(page: &Value) -> Vec<&str> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["package"].as_str().unwrap())
        .collect()
}
fn rating(level: Option<Level>) -> Rating {
    Rating {
        level,
        source: None,
        description: String::new(),
        dependencies: vec![],
        needed_by: vec![],
    }
}

#[test]
fn group_for_covers_every_case() {
    for (level, group) in [
        (Some(Level::Recommended), Group::Recommended),
        (Some(Level::Advanced), Group::Advanced),
        (None, Group::NotRated),
        (Some(Level::Expert), Group::Expert),
        (Some(Level::Unsafe), Group::Unsafe),
    ] {
        assert_eq!(group_for(Some(&rating(level)), false), group);
    }
    assert_eq!(group_for(None, false), Group::NotRated);
    assert_eq!(group_for(None, true), Group::Installed);
    assert_eq!(
        group_for(Some(&rating(Some(Level::Unsafe))), true),
        Group::Installed
    );
}

#[test]
fn groups_sort_in_the_agreed_order() {
    let mut groups = vec![
        Group::Unsafe,
        Group::Expert,
        Group::NotRated,
        Group::Advanced,
        Group::Recommended,
        Group::Installed,
    ];
    groups.sort();
    assert_eq!(groups[0], Group::Installed);
    assert_eq!(groups[3], Group::NotRated);
    assert_eq!(groups[5], Group::Unsafe);
}

#[tokio::test]
async fn orders_groups_then_label_then_package() {
    let list = ratings();
    let page = query_rated(
        &mut fake(),
        &target(),
        packages_query("all", "", 0, 32),
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(
        ids(&page),
        [
            "com.mine",
            "com.mine.rated",
            "com.good.two",
            "com.good",
            "com.mid",
            "com.odd",
            "com.unknown",
            "com.hard",
            "com.risky",
        ]
    );
    assert_eq!(page["items"][0]["group"], "installed");
    assert_eq!(page["items"][1]["rating"]["level"], "unsafe");
    assert_eq!(page["items"][6]["group"], "notRated");
    assert_eq!(page["items"][6]["rating"], Value::Null);
}

#[tokio::test]
async fn counts_cover_the_whole_filtered_result() {
    let list = ratings();
    let page = query_rated(
        &mut fake(),
        &target(),
        packages_query("all", "", 0, 2),
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(ids(&page).len(), 2);
    assert_eq!(
        page["groups"],
        json!([
            {"id":"installed","count":2},
            {"id":"recommended","count":2},
            {"id":"advanced","count":1},
            {"id":"notRated","count":2},
            {"id":"expert","count":1},
            {"id":"unsafe","count":1},
        ])
    );
}

#[tokio::test]
async fn search_filters_before_counting() {
    let list = ratings();
    let page = query_rated(
        &mut fake(),
        &target(),
        packages_query("all", "risky", 0, 32),
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(page["groups"], json!([{"id":"unsafe","count":1}]));
}

#[tokio::test]
async fn user_filter_only_has_the_installed_group() {
    let list = ratings();
    let mut d = fake();
    d.all = vec!["com.mine", "com.mine.rated"];
    let page = query_rated(
        &mut d,
        &target(),
        packages_query("user", "", 0, 32),
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(page["groups"], json!([{"id":"installed","count":2}]));
    assert_eq!(
        d.commands
            .iter()
            .filter(|c| c.starts_with("pm list"))
            .count(),
        1
    );
}

#[tokio::test]
async fn failed_user_lookup_falls_back_to_rating_groups() {
    let list = ratings();
    let mut d = fake();
    d.fail_user = true;
    let page = query_rated(
        &mut d,
        &target(),
        packages_query("all", "", 0, 32),
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(page["items"][0]["group"], "recommended");
    assert!(page["metadataNote"].is_string());
}

#[tokio::test]
async fn no_list_keeps_todays_shape() {
    let mut d = fake();
    let page = query_rated(&mut d, &target(), packages_query("all", "", 0, 32), None)
        .await
        .unwrap();
    assert!(page.get("groups").is_none());
    assert!(page["items"][0].get("group").is_none());
    assert_eq!(ids(&page)[0], "com.good.two");
    assert_eq!(
        d.commands
            .iter()
            .filter(|c| c.starts_with("pm list"))
            .count(),
        1
    );
}

#[tokio::test]
async fn package_detail_includes_its_rating_and_marks_related_packages_on_this_tv() {
    let list = ratings();
    let detail = query_rated(
        &mut fake(),
        &target(),
        Query::Package {
            package: "com.risky".into(),
        },
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(detail["rating"]["level"], "unsafe");
    assert_eq!(detail["rating"]["description"], "Vital");
    assert_eq!(detail["rating"]["source"], "Aosp");
    assert_eq!(detail["rating"]["neededBy"], json!(["com.hard"]));
    assert_eq!(detail["installedRelated"], json!(["com.hard", "com.mid"]));
}

#[tokio::test]
async fn package_detail_makes_no_extra_call_without_related_packages() {
    let list = ratings();
    let mut d = fake();
    let detail = query_rated(
        &mut d,
        &target(),
        Query::Package {
            package: "com.good".into(),
        },
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(detail["rating"]["level"], "recommended");
    assert!(!d.commands.iter().any(|c| c.starts_with("pm list")));
}

#[tokio::test]
async fn an_unlisted_package_has_no_rating() {
    let list = ratings();
    let detail = query_rated(
        &mut fake(),
        &target(),
        Query::Package {
            package: "com.unknown".into(),
        },
        Some(&list),
    )
    .await
    .unwrap();
    assert_eq!(detail["rating"], Value::Null);
}

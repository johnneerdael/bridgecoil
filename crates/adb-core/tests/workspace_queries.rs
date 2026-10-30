use adb_core::workspace::queries::{parse_components, parse_packages, parse_profiles};
#[test]
fn inventory_ignores_oem_noise_and_preserves_ids() {
    let p = parse_packages(
        "warning: vendor message\npackage:com.example.player uid:10123\npackage:android uid:1000\npackage:com.bad;id\npackage:com.example.player\n",
    );
    assert_eq!(p, vec!["android", "com.example.player"]);
}
#[test]
fn profile_names_may_contain_colons() {
    let p = parse_profiles("Users:\n UserInfo{0:Owner:13} running\n UserInfo{10:Kids: TV:10}\n");
    assert_eq!(p.len(), 2);
    assert_eq!(p[1].id, 10);
    assert_eq!(p[1].name, "Kids: TV");
}
#[test]
fn only_valid_activity_components_become_launch_targets() {
    let p = parse_components(
        "2 activities found:\norg.app/.Main\norg.app/org.app.Other\norg.bad/.Main;reboot\n",
    );
    assert_eq!(p.len(), 2);
}

use adb_core::ratings::{Level, RatingList, check_acceptable, parse_list};
use serde_json::{Map, Value, json};

fn sample_list(recommended: usize, unsafe_count: usize) -> String {
    let mut entries = Map::new();
    for index in 0..recommended {
        entries.insert(
            format!("com.good.{index}"),
            json!({"removal":"Recommended"}),
        );
    }
    for index in 0..unsafe_count {
        entries.insert(format!("com.risky.{index}"), json!({"removal":"Unsafe"}));
    }
    Value::Object(entries).to_string()
}
fn list(recommended: usize, unsafe_count: usize) -> RatingList {
    parse_list(&sample_list(recommended, unsafe_count)).unwrap()
}

#[test]
fn parses_all_fields() {
    let list = parse_list(
        r#"{"com.a":{"list":"Oem","description":"Launcher\nfor TVs","dependencies":["com.b"],
        "neededBy":["com.c"],"labels":[],"removal":"Expert"}}"#,
    )
    .unwrap();
    let rating = list.get("com.a").unwrap();
    assert_eq!(rating.level, Some(Level::Expert));
    assert_eq!(rating.source.as_deref(), Some("Oem"));
    assert_eq!(rating.description, "Launcher\nfor TVs");
    assert_eq!(rating.dependencies, ["com.b"]);
    assert_eq!(rating.needed_by, ["com.c"]);
}

#[test]
fn levels_ignore_case() {
    let list = parse_list(r#"{"a":{"removal":"unsafe"},"b":{"removal":"UNSAFE"}}"#).unwrap();
    assert_eq!(list.get("a").unwrap().level, Some(Level::Unsafe));
    assert_eq!(list.get("b").unwrap().level, Some(Level::Unsafe));
}

#[test]
fn unknown_levels_are_not_rated() {
    let list = parse_list(r#"{"a":{"removal":"Unlisted"},"b":{"removal":""},"c":{"list":"Oem"}}"#)
        .unwrap();
    for package in ["a", "b", "c"] {
        assert_eq!(list.get(package).unwrap().level, None, "{package}");
    }
    assert_eq!(list.recognised(), 0);
}

#[test]
fn wrongly_typed_fields_are_treated_as_missing() {
    let list = parse_list(
        r#"{"a":{"removal":5,"description":7,"dependencies":"x","neededBy":null,"list":[]}}"#,
    )
    .unwrap();
    let rating = list.get("a").unwrap();
    assert_eq!(rating.level, None);
    assert_eq!(rating.source, None);
    assert_eq!(rating.description, "");
    assert!(rating.dependencies.is_empty());
    assert!(rating.needed_by.is_empty());
}

#[test]
fn non_string_array_items_are_dropped() {
    let list = parse_list(r#"{"a":{"dependencies":["b",1]}}"#).unwrap();
    assert_eq!(list.get("a").unwrap().dependencies, ["b"]);
}

#[test]
fn non_object_entries_are_skipped() {
    let list = parse_list(r#"{"a":3,"b":{"removal":"Expert"}}"#).unwrap();
    assert!(list.get("a").is_none());
    assert!(list.get("b").is_some());
}

#[test]
fn invalid_json_is_an_error() {
    assert!(parse_list("{not json").is_err());
}

#[test]
fn a_top_level_array_is_an_error() {
    assert!(parse_list("[]").is_err());
}

#[test]
fn unknown_fields_are_ignored() {
    let list = parse_list(r#"{"a":{"removal":"Advanced","leabel":"x","labelid":1}}"#).unwrap();
    assert_eq!(list.get("a").unwrap().level, Some(Level::Advanced));
}

#[test]
fn accepts_a_normal_list() {
    assert!(check_acceptable(&list(999, 1), None).is_ok());
}

#[test]
fn rejects_too_few_recognised() {
    let e = check_acceptable(&list(998, 1), None).unwrap_err();
    assert_eq!(e.kind, "ratingsRejected");
}

#[test]
fn rejects_no_unsafe() {
    let e = check_acceptable(&list(2000, 0), None).unwrap_err();
    assert_eq!(e.kind, "ratingsRejected");
}

#[test]
fn rejects_losing_most_of_the_stored_list() {
    let e = check_acceptable(&list(1498, 1), Some(&list(2999, 1))).unwrap_err();
    assert_eq!(e.kind, "ratingsRejected");
}

#[test]
fn accepts_exactly_half_the_stored_list() {
    assert!(check_acceptable(&list(1499, 1), Some(&list(2999, 1))).is_ok());
}

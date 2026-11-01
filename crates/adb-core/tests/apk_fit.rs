//! A person who gets refused has to go and find a different file, so the message has to say
//! what this file is and what the TV needs.
use adb_core::apk::{ApkInfo, unfit, wrong_processor};

fn list(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

fn apk(min_sdk: Option<u32>, abis: &[&str]) -> ApkInfo {
    ApkInfo {
        min_sdk,
        abis: list(abis),
        ..Default::default()
    }
}

#[test]
fn a_64_bit_file_on_32_bit_android_names_both_and_what_to_find() {
    let tv = list(&["armeabi-v7a", "armeabi"]);
    let message = unfit(&apk(Some(24), &["arm64-v8a"]), 28, &tv).unwrap();
    assert_eq!(
        message,
        "This file is for 64-bit ARM (arm64-v8a), but this TV needs 32-bit ARM (armeabi-v7a). \
         Look for the armeabi-v7a version of this app. Many TVs have a 64-bit chip but run 32-bit \
         Android, and it's Android that counts."
    );
}

#[test]
fn the_first_sentence_fits_the_two_line_banner() {
    let message = wrong_processor(
        &list(&["arm64-v8a"]),
        &list(&["armeabi-v7a", "armeabi"]),
    )
    .unwrap();
    let first = message.split(". ").next().unwrap();
    assert!(first.len() <= 90, "{first}");
}

#[test]
fn a_32_bit_file_on_a_64_bit_only_tv_leaves_out_the_chip_note() {
    let message = wrong_processor(&list(&["armeabi-v7a"]), &list(&["arm64-v8a"])).unwrap();
    assert!(message.contains("needs 64-bit ARM (arm64-v8a)"), "{message}");
    assert!(!message.contains("chip"), "{message}");
}

#[test]
fn each_processor_type_is_named_once() {
    let message = wrong_processor(
        &list(&["armeabi-v7a", "armeabi", "x86_64"]),
        &list(&["arm64-v8a"]),
    )
    .unwrap();
    assert!(
        message.contains("32-bit ARM and 64-bit Intel (armeabi-v7a, armeabi, x86_64)"),
        "{message}"
    );
}

#[test]
fn a_processor_it_has_no_name_for_is_still_shown() {
    let message = wrong_processor(&list(&["riscv64"]), &list(&["arm64-v8a"])).unwrap();
    assert!(message.contains("for riscv64 (riscv64)"), "{message}");
}

#[test]
fn a_new_app_on_an_old_tv_names_both_android_versions() {
    let message = unfit(&apk(Some(31), &[]), 28, &[]).unwrap();
    assert_eq!(
        message,
        "This app needs Android 12 or newer, and this TV has Android 9. Look for an older version \
         of this app."
    );
    let future = unfit(&apk(Some(99), &[]), 28, &[]).unwrap();
    assert!(future.contains("Android API level 99"), "{future}");
}

#[test]
fn a_file_that_runs_here_passes() {
    let tv = list(&["arm64-v8a", "armeabi-v7a", "armeabi"]);
    assert_eq!(unfit(&apk(Some(24), &["armeabi-v7a"]), 28, &tv), None);
    assert_eq!(unfit(&apk(Some(28), &[]), 28, &tv), None);
    assert_eq!(unfit(&apk(None, &["x86", "armeabi"]), 28, &tv), None);
}

#[test]
fn a_tv_that_could_not_be_read_leaves_the_decision_to_android() {
    assert_eq!(unfit(&apk(Some(99), &["x86"]), 0, &[]), None);
}

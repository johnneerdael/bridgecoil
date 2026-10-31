use adb_core::workspace::queries::{eligible_homes, parse_screenshot, screenshot_command};

#[test]
fn a_real_png_comes_back_clean() {
    let png = "iVBORw0KGgoAAAANSUhEUg==\n";
    assert_eq!(parse_screenshot(png).unwrap(), "iVBORw0KGgoAAAANSUhEUg==");
}

#[test]
fn line_wrapped_output_is_joined() {
    assert_eq!(
        parse_screenshot("iVBORw0K\nGgoAAAAN\r\n").unwrap(),
        "iVBORw0KGgoAAAAN"
    );
}

#[test]
fn anything_that_is_not_a_png_is_an_error() {
    assert_eq!(parse_screenshot("").unwrap_err().kind, "captureUnavailable");
    assert!(parse_screenshot("Error: permission denied").is_err());
}

#[test]
fn the_image_leaves_nothing_behind_on_the_tv() {
    let command = screenshot_command();
    assert!(command.contains("screencap -p"));
    assert!(command.contains("base64"));
    assert!(!command.contains("/sdcard"));
}

#[test]
fn settings_is_never_offered_as_a_launcher() {
    let homes = eligible_homes(vec![
        "com.android.tv.settings/.FallbackHome".into(),
        "com.android.settings/.FallbackHome".into(),
        "com.vendor.home/.Main".into(),
    ]);
    assert_eq!(homes, ["com.vendor.home/.Main"]);
}

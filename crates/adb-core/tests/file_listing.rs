use adb_core::workspace::files::{
    FileKind, FolderStatus, MAX_ENTRIES, listing_command, parse_listing,
};

fn entry(listing: &adb_core::workspace::files::FileListing, name: &str) -> usize {
    listing.entries.iter().position(|e| e.name == name).unwrap()
}

#[test]
fn a_partly_denied_root_still_lists_and_marks_each_entry() {
    let raw = "ok\0w\0de-r-xsystem\0o-l---stale\0o-----init\0d-----data_mirror\0:meta\0\
               BC|4096|1700000000|./system\n";
    let listing = parse_listing("/", raw).unwrap();
    assert_eq!(listing.status, FolderStatus::Ok);
    assert!(listing.writable);
    assert_eq!(listing.total, 4);
    let system = &listing.entries[entry(&listing, "system")];
    assert_eq!(system.kind, FileKind::Folder);
    assert_eq!(system.path, "/system");
    assert!(system.can_open);
    assert_eq!(system.modified, Some(1700000000));
    assert_eq!(
        listing.entries[entry(&listing, "stale")].kind,
        FileKind::BrokenLink
    );
    for name in ["init", "data_mirror"] {
        let denied = &listing.entries[entry(&listing, name)];
        assert_eq!(denied.kind, FileKind::Denied);
        assert!(!denied.can_open);
        assert_eq!(denied.size, None);
    }
}

#[test]
fn links_to_folders_open_and_traversal_only_folders_do_not() {
    let raw = "ok\0w\0delr-xsdcard\0de-r-xmusic\0de---xdata\0:meta\0";
    let listing = parse_listing("/", raw).unwrap();
    let sdcard = &listing.entries[entry(&listing, "sdcard")];
    assert_eq!(sdcard.kind, FileKind::Folder);
    assert!(sdcard.link && sdcard.can_open);
    assert!(listing.entries[entry(&listing, "music")].can_open);
    assert!(!listing.entries[entry(&listing, "data")].can_open);
}

#[test]
fn sorts_folders_first_ignores_case_and_keeps_hidden_names_last() {
    let raw = "ok\0w\0fe-r--b.txt\0de-r-xzoo\0de-r-x.cache\0fe-r--A.txt\0de-r-xAlpha\0:meta\0";
    let names: Vec<_> = parse_listing("/sdcard", raw)
        .unwrap()
        .entries
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, ["Alpha", "zoo", ".cache", "A.txt", "b.txt"]);
}

#[test]
fn names_with_newlines_pipes_and_spaces_keep_their_details() {
    let raw = "ok\0w\0fe-r--a\nb|c d.mp4\0fe-r--plain\0:meta\0\
               BC|12|1700000000|./a\nb|c d.mp4\nBC|34|1700000001|./plain\n";
    let listing = parse_listing("/sdcard/Movies", raw).unwrap();
    let odd = &listing.entries[entry(&listing, "a\nb|c d.mp4")];
    assert_eq!(odd.path, "/sdcard/Movies/a\nb|c d.mp4");
    assert_eq!((odd.size, odd.modified), (Some(12), Some(1700000000)));
    assert_eq!(listing.entries[entry(&listing, "plain")].size, Some(34));
}

#[test]
fn a_forged_stat_line_cannot_add_a_row() {
    let raw = "ok\0w\0fe-r--real\0:meta\0BC|1|2|./real\nBC|9|9|./ghost\n";
    let listing = parse_listing("/sdcard", raw).unwrap();
    assert_eq!(listing.total, 1);
    assert_eq!(listing.entries[0].name, "real");
}

#[test]
fn folders_that_cannot_be_read_are_a_status_not_an_error() {
    for (raw, status) in [
        ("denied\0", FolderStatus::Denied),
        ("missing\0", FolderStatus::Missing),
        ("notDirectory\0", FolderStatus::NotDirectory),
    ] {
        let listing = parse_listing("/data", raw).unwrap();
        assert_eq!(listing.status, status);
        assert!(!listing.writable);
        assert!(listing.entries.is_empty());
    }
}

#[test]
fn unusable_output_is_an_error_rather_than_an_empty_folder() {
    assert!(parse_listing("/sdcard", "").is_err());
    assert!(parse_listing("/sdcard", "sh: print: not found").is_err());
}

#[test]
fn large_folders_report_truncation() {
    let mut raw = String::from("ok\0w\0");
    for index in 0..MAX_ENTRIES + 1 {
        raw.push_str(&format!("fe-r--{index}\0"));
    }
    raw.push_str(":meta\0");
    let listing = parse_listing("/sdcard", &raw).unwrap();
    assert_eq!(listing.entries.len(), MAX_ENTRIES);
    assert_eq!(listing.total, MAX_ENTRIES + 1);
    assert!(listing.truncated);
}

#[test]
fn the_path_is_quoted_as_one_argument() {
    let path = "/sdcard/$(reboot)'; reboot;'";
    let command = listing_command(path);
    assert_eq!(
        command.lines().next().unwrap(),
        format!("set -- {}", adb_core::shell_quote(path))
    );
}

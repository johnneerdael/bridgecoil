use adb_core::{
    remote::{
        capture::{KeyIdentity, capture_command, parse_capture},
        mappings::{Mapping, apply_command, listener_script, parse_state, validate},
    },
    shell_quote,
};

const CAPTURE: &str = r#"add device 1: /dev/input/event0
  name:     "gpio-keys"
add device 2: /dev/input/event4
  name:     "Vendor Remote 01"
add device 3: /dev/input/event5
  name:     "touch panel"
could not get driver version for /dev/input/mice, Not a typewriter
/dev/input/event4: EV_MSC       MSC_SCAN             000c0057
/dev/input/event4: EV_KEY       KEY_UNKNOWN          DOWN
/dev/input/event4: EV_SYN       SYN_REPORT           00000000
/dev/input/event4: EV_KEY       KEY_UNKNOWN          REPEAT
/dev/input/event4: EV_SYN       SYN_REPORT           00000000
/dev/input/event4: EV_MSC       MSC_SCAN             000c0057
/dev/input/event4: EV_KEY       KEY_UNKNOWN          UP
/dev/input/event4: EV_SYN       SYN_REPORT           00000000
/dev/input/event4: EV_MSC       MSC_SCAN             000c0051
/dev/input/event4: EV_KEY       KEY_UNKNOWN          DOWN
/dev/input/event4: EV_SYN       SYN_REPORT           00000000
/dev/input/event5: EV_KEY       BTN_TOUCH            DOWN
/dev/input/event0: EV_KEY       KEY_VOLUMEUP         DOWN
/dev/input/event0: EV_SYN       SYN_REPORT           00000000
/dev/input/event0: EV_KEY       0231                 DOWN
"#;

fn key(source: &str, key: &str, scan: Option<&str>) -> KeyIdentity {
    KeyIdentity {
        source: source.into(),
        key: key.into(),
        scan: scan.map(str::to_owned),
    }
}
fn mapping(identity: KeyIdentity, component: &str) -> Mapping {
    Mapping {
        key: identity,
        component: component.into(),
        label: "Player".into(),
    }
}
fn remote(scan: &str) -> KeyIdentity {
    key("Vendor Remote 01", "KEY_UNKNOWN", Some(scan))
}

#[test]
fn buttons_with_the_same_key_name_stay_distinct_by_scan_code() {
    let presses = parse_capture(CAPTURE);
    assert_eq!(presses[0], remote("000c0057"));
    assert_eq!(presses[1], remote("000c0051"));
}

#[test]
fn a_held_button_counts_once() {
    let presses = parse_capture(CAPTURE);
    assert_eq!(
        presses.iter().filter(|p| **p == remote("000c0057")).count(),
        1
    );
}

#[test]
fn touch_events_are_not_buttons() {
    assert!(
        parse_capture(CAPTURE)
            .iter()
            .all(|p| p.source != "touch panel")
    );
}

#[test]
fn names_and_numbers_are_kept_as_the_tv_reported_them() {
    let presses = parse_capture(CAPTURE);
    assert!(presses.contains(&key("gpio-keys", "KEY_VOLUMEUP", None)));
    assert!(presses.contains(&key("gpio-keys", "0231", None)));
}

#[test]
fn a_scan_code_does_not_leak_onto_the_next_device() {
    let raw = "add device 1: /dev/input/event1\n  name:     \"A\"\n\
add device 2: /dev/input/event2\n  name:     \"B\"\n\
/dev/input/event1: EV_MSC       MSC_SCAN             00000001\n\
/dev/input/event2: EV_KEY       KEY_A                DOWN\n";
    assert_eq!(parse_capture(raw), [key("B", "KEY_A", None)]);
}

#[test]
fn a_key_from_an_unnamed_device_is_ignored() {
    assert!(
        parse_capture("/dev/input/event9: EV_KEY       KEY_A                DOWN\n").is_empty()
    );
}

#[test]
fn capture_is_bounded() {
    assert!(capture_command(8).unwrap().contains("timeout 8 getevent -l"));
    assert!(capture_command(0).is_err());
    assert!(capture_command(25).is_err());
}

#[test]
fn a_valid_set_generates_a_listener() {
    let script = listener_script(&[
        mapping(remote("000c0057"), "org.demo.player/.Main"),
        mapping(
            key("gpio-keys", "KEY_F1", None),
            "org.demo.other/org.demo.Other",
        ),
    ])
    .unwrap();
    assert!(script.contains("000c0057"));
    assert!(script.contains("KEY_F1"));
    assert!(script.contains(&shell_quote("Vendor Remote 01")));
    assert!(!script.contains("@@"));
    assert!(!script.contains("eval"));
}

#[test]
fn hostile_device_names_stay_quoted() {
    let name = "x'; reboot; echo '";
    let script = listener_script(&[mapping(key(name, "KEY_A", None), "org.demo/.Main")]).unwrap();
    assert!(script.contains(&shell_quote(name)));
}

#[test]
fn bad_fields_are_refused_before_anything_runs() {
    for bad in [
        mapping(key("a", "KEY_A; reboot", None), "org.demo/.Main"),
        mapping(key("a", "KEY_A", Some("00$(id)")), "org.demo/.Main"),
        mapping(key("a", "KEY_A", None), "org.demo/.Main; reboot"),
        mapping(key("a\nb", "KEY_A", None), "org.demo/.Main"),
        mapping(key("", "KEY_A", None), "org.demo/.Main"),
    ] {
        assert!(validate(std::slice::from_ref(&bad)).is_err(), "{bad:?}");
    }
}

#[test]
fn one_button_can_only_do_one_thing() {
    let e = validate(&[
        mapping(remote("000c0057"), "org.demo/.Main"),
        mapping(remote("000c0057"), "org.other/.Main"),
    ])
    .unwrap_err();
    assert_eq!(e.kind, "mappingConflict");
}

#[test]
fn an_empty_set_is_refused_so_removal_stays_explicit() {
    assert!(validate(&[]).is_err());
}

#[test]
fn apply_stores_the_set_on_the_tv() {
    let command = apply_command(&[mapping(remote("000c0057"), "org.demo/.Main")]).unwrap();
    assert!(command.contains("/data/local/tmp/bridgecoil/mappings.json"));
    assert!(command.contains("listener.sh --start"));
    assert!(
        command.contains("chmod 700 /data/local/tmp/bridgecoil"),
        "no other app may edit what runs as the shell user"
    );
}

#[test]
fn reads_back_what_the_tv_holds() {
    let set = vec![mapping(remote("000c0057"), "org.demo/.Main")];
    let raw = format!(
        "running=1\npaused=\n---mappings\n{}\n---log\n1 fired\n2 fired\n",
        serde_json::to_string(&set).unwrap()
    );
    let state = parse_state(&raw);
    assert!(state.running);
    assert!(!state.paused);
    assert_eq!(state.mappings, set);
    assert_eq!(state.log, ["1 fired", "2 fired"]);
}

#[test]
fn a_tv_with_nothing_set_up_reads_as_empty() {
    let state = parse_state("running=\npaused=\n---mappings\n---log\n");
    assert!(!state.running);
    assert!(state.mappings.is_empty());
    assert!(state.log.is_empty());
}

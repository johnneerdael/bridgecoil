use adb_core::workspace::queries::{
    RECORDING_PATH, RecordingStep, logs_command, recording_command,
};

#[test]
fn logs_are_filtered_by_how_much_detail_was_asked_for() {
    let command = logs_command("warning", None, 200).unwrap();
    assert!(command.contains("logcat -d -t 200"));
    assert!(command.contains("'*:W'"));
}

#[test]
fn an_app_narrows_logs_to_its_process() {
    let command = logs_command("error", Some("org.demo.player"), 1000).unwrap();
    assert!(command.contains("pidof 'org.demo.player'"));
    assert!(command.contains("--pid=\"$pid\""));
}

#[test]
fn log_choices_are_never_passed_through() {
    assert!(logs_command("*:V; reboot", None, 200).is_err());
    assert!(logs_command("error", Some("org.demo; reboot"), 200).is_err());
    assert!(logs_command("error", None, 0).is_err());
    assert!(logs_command("error", None, 1_000_000).is_err());
}

#[test]
fn a_recording_has_an_end_and_stays_out_of_the_owners_media() {
    let command = recording_command(RecordingStep::Start { seconds: 30 }).unwrap();
    assert!(command.contains("--time-limit 30"));
    assert!(RECORDING_PATH.starts_with("/data/local/tmp/"));
    assert!(recording_command(RecordingStep::Start { seconds: 0 }).is_err());
    assert!(recording_command(RecordingStep::Start { seconds: 181 }).is_err());
}

#[test]
fn stopping_lets_the_recording_finish_its_file() {
    assert!(
        recording_command(RecordingStep::Stop)
            .unwrap()
            .contains("kill -INT")
    );
}

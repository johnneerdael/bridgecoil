//! Runs the real monitor script with stand-ins for the tools it calls, so its give-up logic is
//! tested on every build, not only when a device is attached.
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command, time::Duration};

const SCRIPT: &str = include_str!("../src/toolkit/scripts/boot_monitor.sh");

fn executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
/// Returns the log. `shell` is the fake manager shell, and the run ends after `naps` sleeps.
fn run_monitor(dir: &Path, shell: &str, naps: u32) -> String {
    let home = dir.join("home");
    let bin = dir.join("bin");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&bin).unwrap();
    executable(&home.join("shell"), shell);
    executable(&bin.join("timeout"), "#!/bin/sh\nshift\nexec \"$@\"\n");
    executable(
        &bin.join("sleep"),
        &format!(
            "#!/bin/sh\nn=$(cat {0}/naps 2>/dev/null || echo 0)\nn=$((n + 1))\necho $n > {0}/naps\n\
             [ \"$n\" -ge {naps} ] && kill $PPID\nexit 0\n",
            dir.display()
        ),
    );
    let script = dir.join("00-bridgecoil");
    let body = SCRIPT
        .replace(
            "/data/data/com.termux/files/home/.bridgecoil",
            &home.display().to_string(),
        )
        .replace("#!/data/data/com.termux/files/usr/bin/sh", "#!/bin/sh");
    executable(&script, &body);
    let path = format!("{}:/usr/bin:/bin", bin.display());
    let mut child = Command::new("/bin/sh")
        .arg(&script)
        .env("PATH", path)
        .spawn()
        .unwrap();
    // A busy machine can take this long to start the stand-in tools.
    for _ in 0..600 {
        if child.try_wait().unwrap().is_some() {
            return fs::read_to_string(home.join("monitor.log")).unwrap_or_default();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.kill().unwrap();
    panic!("the monitor never stopped");
}

#[test]
fn it_stops_after_three_refusals_so_the_tv_is_not_nagged() {
    let dir = tempfile::tempdir().unwrap();
    let log = run_monitor(dir.path(), "#!/bin/sh\necho denied\nexit 1\n", 50);
    assert_eq!(log.lines().count(), 3, "{log}");
    assert!(log.contains("(3 of 3)"));
    assert_eq!(
        fs::read_to_string(dir.path().join("naps")).unwrap().trim(),
        "2"
    );
}

#[test]
fn it_keeps_waiting_for_a_manager_that_has_not_started() {
    let dir = tempfile::tempdir().unwrap();
    let log = run_monitor(
        dir.path(),
        "#!/bin/sh\necho Server is not running\nexit 1\n",
        8,
    );
    assert_eq!(log, "", "waiting is not a refusal");
    assert_eq!(
        fs::read_to_string(dir.path().join("naps")).unwrap().trim(),
        "8"
    );
}

#[test]
fn a_recovered_manager_clears_earlier_refusals() {
    let dir = tempfile::tempdir().unwrap();
    let flaky = format!(
        "#!/bin/sh\nn=$(cat {0}/calls 2>/dev/null || echo 0)\nn=$((n + 1))\necho $n > {0}/calls\n\
         [ $((n % 3)) -eq 0 ] && exit 0\necho denied\nexit 1\n",
        dir.path().display()
    );
    let log = run_monitor(dir.path(), &flaky, 12);
    assert!(!log.contains("(3 of 3)"), "{log}");
}

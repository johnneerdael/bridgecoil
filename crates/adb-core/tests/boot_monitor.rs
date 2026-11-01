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
    executable(&home.join("agent"), shell);
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
fn it_keeps_asking_until_the_tv_offers_adb() {
    let dir = tempfile::tempdir().unwrap();
    let agent = format!(
        "#!/bin/sh\nn=$(cat {0}/calls 2>/dev/null || echo 0)\necho $((n + 1)) > {0}/calls\nexit 1\n",
        dir.path().display()
    );
    run_monitor(dir.path(), &agent, 8);
    let calls = fs::read_to_string(dir.path().join("calls")).unwrap();
    assert_eq!(calls.trim(), "8", "a slow start must not stop the monitor");
}

#[test]
fn it_keeps_checking_after_the_listener_starts() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("args");
    let agent = format!("#!/bin/sh\necho \"$@\" >> {}\nexit 0\n", log.display());
    run_monitor(dir.path(), &agent, 3);
    let calls = fs::read_to_string(log).unwrap();
    assert_eq!(calls.lines().count(), 3, "{calls}");
    assert!(
        calls.contains("run [ ! -f /data/local/tmp/bridgecoil/boot.sh ] || sh /data/local/tmp/bridgecoil/boot.sh"),
        "{calls}"
    );
}

/// Bridgecoil can replace `boot.sh` over ADB, so what runs at boot never needs Termux again.
#[test]
fn the_monitor_leaves_every_job_to_boot_sh() {
    assert!(!SCRIPT.contains("listener.sh"));
}

/// Runs the real `boot.sh` in `dir`, and returns how it ended.
fn run_boot(dir: &Path) -> std::process::ExitStatus {
    let script = dir.join("boot.sh");
    let body = adb_core::remote::mappings::BOOT_SCRIPT
        .replace("/data/local/tmp/bridgecoil", &dir.display().to_string());
    executable(&script, &body);
    Command::new("/bin/sh").arg(&script).status().unwrap()
}

#[test]
fn boot_sh_starts_the_listener() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("args");
    executable(
        &dir.path().join("listener.sh"),
        &format!("#!/bin/sh\necho \"$@\" >> {}\nexit 0\n", log.display()),
    );
    assert!(run_boot(dir.path()).success());
    assert_eq!(fs::read_to_string(log).unwrap().trim(), "--start");
}

/// A TV can finish setup before anything is mapped, and that isn't a failure to retry.
#[test]
fn boot_sh_has_nothing_to_do_without_mappings() {
    let dir = tempfile::tempdir().unwrap();
    assert!(run_boot(dir.path()).success());
}

/// A failed start has to reach the monitor, which then retries in seconds instead of a minute.
#[test]
fn boot_sh_reports_a_listener_that_fails() {
    let dir = tempfile::tempdir().unwrap();
    executable(&dir.path().join("listener.sh"), "#!/bin/sh\nexit 3\n");
    assert!(!run_boot(dir.path()).success());
}

//! Walks the whole guided setup on a disposable device. Run by hand against an emulator only:
//! BC_ENDPOINT=127.0.0.1:5611 BC_MANAGER=next cargo test -p adb-core --test toolkit_live -- --ignored --nocapture
use adb_core::{
    backend::Client,
    toolkit::{
        catalog::ToolId,
        inventory::{Manager, Stage, inspect},
        release::{GithubReleases, fetch_tool},
        setup::SetupStep,
    },
    workspace::{
        driver::ShellDriver,
        engine::{Action, PackageAction, Workspace},
        history::History,
        queries::{Query, RECORDING_PATH, RecordingStep, query},
        types::{OperationStatus, Target},
    },
};
use std::time::Duration;

/// A secure TV prompts for every new key, so a run can keep one it has already been allowed.
fn identity(dir: &std::path::Path) -> std::path::PathBuf {
    std::env::var_os("BC_IDENTITY").map_or_else(|| dir.join("identity"), Into::into)
}
async fn install(c: &mut Client, id: ToolId, dir: &std::path::Path) {
    let info = c.status().unwrap();
    let abis = info.abis.clone();
    let dest = dir.to_owned();
    let tool = tokio::task::spawn_blocking(move || {
        fetch_tool(&GithubReleases::default(), id, &abis, &dest).unwrap()
    })
    .await
    .unwrap();
    println!(
        "fetched {:?} {} {}",
        id,
        tool.tag,
        tool.info.version_name.as_deref().unwrap_or("?")
    );
    c.install(&info.id, &tool.path, &tool.info, |_| {})
        .await
        .unwrap();
    std::fs::remove_file(&tool.path).unwrap();
}
async fn step(w: &Workspace, c: &mut Client, t: &Target, s: SetupStep) -> OperationStatus {
    let review = w.prepare(c, t, Action::Setup { step: s }).await.unwrap();
    let done = w.apply(c, &review.id).await.unwrap();
    println!(
        "{} -> {:?} {} {}",
        review.title,
        done.status,
        done.error.as_deref().unwrap_or(""),
        done.output.trim().replace(char::from(10), " | ")
    );
    done.status
}
async fn stage(c: &mut Client, t: &Target) -> Stage {
    inspect(c, t).await.unwrap().stage
}
/// Stands in for the person at the TV: taps "Always allow" on the prompt.
async fn approve(c: &mut Client, t: &Target) {
    for _ in 0..20 {
        let raw = c
            .shell(
                t,
                "uiautomator dump /sdcard/bc-ui.xml >/dev/null 2>&1; cat /sdcard/bc-ui.xml",
            )
            .await
            .unwrap()
            .stdout;
        let node = raw.split("<node").find(|n| {
            n.contains("text=\"Always allow\"") || n.contains("text=\"Allow all the time\"")
        });
        if std::env::var("BC_DEBUG").is_ok() {
            let texts: Vec<_> = raw
                .split("text=\"")
                .skip(1)
                .map(|t| t.split(char::from(34)).next().unwrap())
                .filter(|t| !t.is_empty())
                .collect();
            println!("-> screen {texts:?}");
        }
        if let Some(node) = node {
            let bounds = node.split("bounds=\"[").nth(1).unwrap();
            let n: Vec<i32> = bounds
                .split(|ch: char| !ch.is_ascii_digit())
                .filter(|p| !p.is_empty())
                .take(4)
                .map(|p| p.parse().unwrap())
                .collect();
            let tap = format!("input tap {} {}", (n[0] + n[2]) / 2, (n[1] + n[3]) / 2);
            c.shell(t, &tap).await.unwrap();
            return;
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
    }
    panic!("no prompt appeared");
}

#[tokio::test]
#[ignore]
async fn walks_every_stage() {
    let dir = tempfile::tempdir().unwrap();
    let manager = match std::env::var("BC_MANAGER").as_deref() {
        Ok("porter") => Manager::Porter,
        _ => Manager::ShizukuNext,
    };
    let mut c = Client::new(identity(dir.path()));
    c.connect(std::env::var("BC_ENDPOINT").unwrap(), true)
        .await
        .unwrap();
    let t = c.current_target().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap());
    assert_eq!(stage(&mut c, &t).await, Stage::Manager);
    install(&mut c, manager.tool(), dir.path()).await;
    assert_eq!(stage(&mut c, &t).await, Stage::Server);
    assert_eq!(
        step(&w, &mut c, &t, SetupStep::StartServer { manager }).await,
        OperationStatus::Verified
    );
    assert_eq!(stage(&mut c, &t).await, Stage::Terminal);
    install(&mut c, ToolId::Termux, dir.path()).await;
    install(&mut c, ToolId::TermuxBoot, dir.path()).await;
    assert_eq!(
        step(&w, &mut c, &t, SetupStep::OpenTerminal).await,
        OperationStatus::Verified
    );
    step(&w, &mut c, &t, SetupStep::AllowBackground).await;
    assert_eq!(stage(&mut c, &t).await, Stage::Shell);
    assert_eq!(
        step(&w, &mut c, &t, SetupStep::DeployShell { manager }).await,
        OperationStatus::Verified
    );
    assert_eq!(stage(&mut c, &t).await, Stage::Access);
    step(&w, &mut c, &t, SetupStep::RequestAccess).await;
    approve(&mut c, &t).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let route = query(&mut c, &t, Query::VerifyShell).await.unwrap();
    println!("route {route}");
    assert_eq!(route["route"], "shell");
    assert_eq!(stage(&mut c, &t).await, Stage::Startup);
    step(&w, &mut c, &t, SetupStep::AllowBootStart { manager }).await;
    assert_eq!(
        step(&w, &mut c, &t, SetupStep::InstallMonitor).await,
        OperationStatus::Verified
    );
    assert_eq!(stage(&mut c, &t).await, Stage::Done);
    println!(
        "snapshot {}",
        serde_json::to_string(&inspect(&mut c, &t).await.unwrap()).unwrap()
    );
}

/// A real kernel key event. The emulator has no remote, so this presses its power button.
fn press() {
    key_down_up("");
}
fn hold() {
    key_down_up("sleep 1;");
}
/// With no root the emulator console is the only way to a real key event, and it can only tap.
fn can_hold() -> bool {
    std::env::var("BC_EMU_KEY").is_err()
}
fn key_down_up(between: &str) {
    let device = std::env::var("BC_DEVICE").unwrap();
    let adb = |args: &[&str]| {
        let done = std::process::Command::new("adb")
            .args(["-s", &device])
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(done.status.success());
    };
    if !can_hold() {
        adb(&["emu", "event", "text", "a"]);
        return;
    }
    let keys = format!(
        "sendevent /dev/input/event0 1 116 1; sendevent /dev/input/event0 0 0 0; {between} \
         sendevent /dev/input/event0 1 116 0; sendevent /dev/input/event0 0 0 0"
    );
    adb(&["shell", "su", "0", "sh", "-c", &format!("'{keys}'")]);
}
async fn wake(c: &mut Client, t: &Target) {
    c.shell(t, "input keyevent KEYCODE_WAKEUP").await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
}
async fn top(c: &mut Client, t: &Target) -> String {
    c.shell(
        t,
        "dumpsys activity activities | grep -m1 topResumedActivity",
    )
    .await
    .unwrap()
    .stdout
}
async fn mapping_state(c: &mut Client, t: &Target) -> serde_json::Value {
    query(c, t, Query::Mappings).await.unwrap()
}

async fn press_home(c: &mut Client, t: &Target) -> String {
    c.shell(t, "input keyevent KEYCODE_HOME").await.unwrap();
    tokio::time::sleep(Duration::from_secs(4)).await;
    top(c, t).await
}
async fn applied(w: &Workspace, c: &mut Client, t: &Target, a: Action) -> OperationStatus {
    let review = w.prepare(c, t, a).await.unwrap();
    let done = w.apply(c, &review.id).await.unwrap();
    println!("{} -> {:?}", review.title, done.status);
    done.status
}

/// Needs a second Home app on the device, named by BC_HOME as package/.Activity. Leaves the TV
/// as it found it.
#[tokio::test]
#[ignore]
async fn a_new_launcher_takes_over_and_gives_way_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::new(identity(dir.path()));
    c.connect(std::env::var("BC_ENDPOINT").unwrap(), true)
        .await
        .unwrap();
    let t = c.current_target().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap());
    let mine = std::env::var("BC_HOME").unwrap();
    let my_package = mine.split('/').next().unwrap().to_owned();

    let homes = query(&mut c, &t, Query::Launchers).await.unwrap();
    println!("{homes}");
    let stock = homes["current"].as_str().unwrap().to_owned();
    let stock_package = stock.split('/').next().unwrap().to_owned();
    assert!(homes["homes"].as_array().unwrap().iter().any(|h| h == &mine));
    assert!(homes["stock"].as_array().unwrap().iter().any(|h| h == &stock));
    assert!(!homes["stock"].as_array().unwrap().iter().any(|h| h == &mine));

    let review = w
        .prepare(
            &mut c,
            &t,
            Action::DefaultHome {
                component: mine.clone(),
            },
        )
        .await
        .unwrap();
    assert!(review.reversible, "changing Home must offer a way back");
    let asked = w.apply(&mut c, &review.id).await.unwrap().status;
    let after = query(&mut c, &t, Query::Launchers).await.unwrap();
    println!("asked {asked:?}: {after}");
    assert_eq!(after["chosen"].as_str(), Some(my_package.as_str()));
    // Android TV's launcher outranks any other, so the choice alone changes nothing.
    assert_eq!(
        asked == OperationStatus::Verified,
        after["current"].as_str() == Some(mine.as_str())
    );
    if asked != OperationStatus::Verified {
        assert!(press_home(&mut c, &t).await.contains(&stock_package));
    }

    let off = Action::Package {
        package: stock_package.clone(),
        action: PackageAction::Disable,
    };
    assert_eq!(
        applied(&w, &mut c, &t, off).await,
        OperationStatus::Verified
    );
    assert!(press_home(&mut c, &t).await.contains(&my_package));
    let refused = w
        .prepare(
            &mut c,
            &t,
            Action::Package {
                package: my_package.clone(),
                action: PackageAction::Disable,
            },
        )
        .await;
    assert!(refused.is_err(), "the launcher in use can't be turned off");
    let left = query(&mut c, &t, Query::Launchers).await.unwrap();
    println!("{left}");
    assert_eq!(left["current"].as_str(), Some(mine.as_str()));

    if std::env::var("BC_KEEP").is_ok() {
        return;
    }
    let on = Action::Package {
        package: stock_package.clone(),
        action: PackageAction::Enable,
    };
    assert_eq!(applied(&w, &mut c, &t, on).await, OperationStatus::Verified);
    let back = Action::DefaultHome {
        component: stock.clone(),
    };
    assert_eq!(
        applied(&w, &mut c, &t, back).await,
        OperationStatus::Verified
    );
    assert!(press_home(&mut c, &t).await.contains(&stock_package));
}

/// Needs walks_every_stage to have finished on this device.
#[tokio::test]
#[ignore]
async fn a_learned_button_opens_its_app() {
    use adb_core::remote::{
        capture::KeyIdentity,
        mappings::{Mapping, MappingChange},
    };
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::new(identity(dir.path()));
    c.connect(std::env::var("BC_ENDPOINT").unwrap(), true)
        .await
        .unwrap();
    let t = c.current_target().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap());
    c.shell(&t, "input keyevent KEYCODE_HOME").await.unwrap();

    let presser = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(2));
        press();
    });
    let result = query(&mut c, &t, Query::CaptureKeys { seconds: 5 })
        .await
        .unwrap();
    presser.join().unwrap();
    println!("captured {}", result["presses"]);
    let key: KeyIdentity = serde_json::from_value(result["presses"][0].clone()).unwrap();
    assert_eq!(
        key.key,
        std::env::var("BC_EMU_KEY").unwrap_or_else(|_| "KEY_POWER".into())
    );
    wake(&mut c, &t).await;
    c.shell(&t, "input keyevent KEYCODE_HOME").await.unwrap();

    let apps = query(&mut c, &t, Query::LaunchableApps).await.unwrap();
    println!("apps {}", apps.as_array().unwrap().len());
    let settings = apps
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["component"].as_str().unwrap().contains(".settings/"))
        .unwrap();
    let mapping = Mapping {
        key: key.clone(),
        hold: false,
        component: settings["component"].as_str().unwrap().into(),
        send: None,
        label: settings["label"].as_str().unwrap().into(),
    };
    let tap = mapping.clone();
    let action = Action::Mappings {
        change: MappingChange::Apply {
            mappings: vec![mapping],
        },
    };
    let review = w.prepare(&mut c, &t, action).await.unwrap();
    println!("{} | {} | {:?}", review.title, review.effect, review.losses);
    let done = w.apply(&mut c, &review.id).await.unwrap();
    println!(
        "apply {:?} {}",
        done.status,
        done.error.as_deref().unwrap_or("")
    );
    assert_eq!(done.status, OperationStatus::Verified);

    press();
    tokio::time::sleep(Duration::from_secs(2)).await;
    wake(&mut c, &t).await;
    let state = mapping_state(&mut c, &t).await;
    println!("log {}", state["log"]);
    let fired = state["log"].as_array().unwrap().len();
    let shown = top(&mut c, &t).await;
    println!("top {}", shown.trim());
    assert!(shown.contains(".settings"));
    assert!(
        state["log"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l.as_str().unwrap().contains(" opened "))
    );

    let pause = Action::Mappings {
        change: MappingChange::Pause { paused: true },
    };
    let review = w.prepare(&mut c, &t, pause).await.unwrap();
    assert_eq!(
        w.apply(&mut c, &review.id).await.unwrap().status,
        OperationStatus::Verified
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    press();
    tokio::time::sleep(Duration::from_secs(2)).await;
    wake(&mut c, &t).await;
    let quiet = mapping_state(&mut c, &t).await;
    assert_eq!(
        quiet["log"].as_array().unwrap().len(),
        fired,
        "a paused button stays quiet"
    );
    let resume = Action::Mappings {
        change: MappingChange::Pause { paused: false },
    };
    let review = w.prepare(&mut c, &t, resume).await.unwrap();
    w.apply(&mut c, &review.id).await.unwrap();

    let both = Action::Mappings {
        change: MappingChange::Apply {
            mappings: vec![
                tap,
                Mapping {
                    key,
                    hold: true,
                    component: String::new(),
                    send: Some("MEDIA_STOP".into()),
                    label: "Stop".into(),
                },
            ],
        },
    };
    let review = w.prepare(&mut c, &t, both).await.unwrap();
    assert_eq!(
        w.apply(&mut c, &review.id).await.unwrap().status,
        OperationStatus::Verified
    );
    if can_hold() {
        hold();
        tokio::time::sleep(Duration::from_secs(2)).await;
        wake(&mut c, &t).await;
        let held = mapping_state(&mut c, &t).await;
        println!("hold log {}", held["log"]);
        assert!(
            held["log"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()
                .as_str()
                .unwrap()
                .ends_with("sent KEYCODE_MEDIA_STOP"),
            "a real long press runs the hold mapping"
        );
    }

    let snap = inspect(&mut c, &t).await.unwrap();
    println!("listener {} monitor {}", snap.listener, snap.monitor);
    assert!(snap.listener);
    assert!(snap.listener);
    if std::env::var("BC_KEEP").is_ok() {
        return;
    }
    let remove = Action::Mappings {
        change: MappingChange::RemoveAll,
    };
    let review = w.prepare(&mut c, &t, remove).await.unwrap();
    assert_eq!(
        w.apply(&mut c, &review.id).await.unwrap().status,
        OperationStatus::Verified
    );
    let left = c.shell(&t, "pidof getevent").await.unwrap().stdout;
    assert_eq!(left.trim(), "", "nothing is left running");
}

#[tokio::test]
#[ignore]
async fn a_screenshot_and_launchers_come_back_whole() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::new(identity(dir.path()));
    c.connect(std::env::var("BC_ENDPOINT").unwrap(), true)
        .await
        .unwrap();
    let t = c.current_target().unwrap();
    let start = std::time::Instant::now();
    let shot = query(&mut c, &t, Query::Screenshot).await.unwrap();
    let png = shot["png"].as_str().unwrap();
    println!(
        "screenshot {} base64 chars in {:?}",
        png.len(),
        start.elapsed()
    );
    use base64::{Engine, engine::general_purpose::STANDARD};
    let bytes = STANDARD.decode(png).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
    assert_eq!(
        &bytes[bytes.len() - 8..bytes.len() - 4],
        b"IEND",
        "the whole image arrived"
    );
    let big = c
        .shell_large(&t, "head -c 6000000 /dev/urandom | base64 | tr -d '\\n'")
        .await
        .unwrap();
    println!("large output {} chars", big.stdout.len());
    assert_eq!(big.stdout.len(), 8000000);
    assert!(c.status().is_some(), "the connection survives it");
    let id = c.status().unwrap().id;
    let path = "/data/local/tmp/bridgecoil-download-test";
    c.shell(&t, &format!("head -c 3000000 /dev/urandom > {path}"))
        .await
        .unwrap();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let mut copied = Vec::new();
    let n = c
        .download(&id, path, &mut copied, &cancel, |_, _| {})
        .await
        .unwrap();
    assert_eq!((n, copied.len()), (3_000_000, 3_000_000));
    let hash = c.shell(&t, &format!("sha256sum {path}")).await.unwrap();
    use sha2::Digest;
    assert!(
        hash.stdout
            .starts_with(&hex::encode(sha2::Sha256::digest(&copied)))
    );
    let denied = c
        .download(
            &id,
            "/data/system/users/0.xml",
            &mut Vec::new(),
            &cancel,
            |_, _| {},
        )
        .await
        .unwrap_err();
    println!("denied {} {}", denied.kind, denied.message);
    assert!(
        c.status().is_some(),
        "a refused file leaves the connection up"
    );
    c.shell(&t, &format!("rm -f {path}")).await.unwrap();

    let start = Query::Recording {
        step: RecordingStep::Start { seconds: 3 },
    };
    query(&mut c, &t, start).await.unwrap();
    tokio::time::sleep(Duration::from_secs(5)).await;
    let done = query(
        &mut c,
        &t,
        Query::Recording {
            step: RecordingStep::Status,
        },
    )
    .await
    .unwrap();
    println!("recording {done}");
    assert_eq!(done["running"], false);
    assert!(done["size"].as_u64().unwrap() > 0);
    let mut video = Vec::new();
    c.download(&id, RECORDING_PATH, &mut video, &cancel, |_, _| {})
        .await
        .unwrap();
    assert_eq!(&video[4..8], b"ftyp", "a playable mp4");
    query(
        &mut c,
        &t,
        Query::Recording {
            step: RecordingStep::Discard,
        },
    )
    .await
    .unwrap();

    let logs = Query::Logs {
        level: "error".into(),
        package: Some("com.android.settings".into()),
        lines: 200,
    };
    let narrowed = query(&mut c, &t, logs).await.unwrap();
    println!("logs not running {}", narrowed["notRunning"]);
    let logs = Query::Logs {
        level: "debug".into(),
        package: None,
        lines: 5000,
    };
    let all = query(&mut c, &t, logs).await.unwrap();
    println!("full logs {} chars", all["output"].as_str().unwrap().len());

    let homes = query(&mut c, &t, Query::Launchers).await.unwrap();
    println!("launchers {homes}");
}

#[tokio::test]
#[ignore]
async fn a_restart_is_noticed() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::new(identity(dir.path()));
    c.connect(std::env::var("BC_ENDPOINT").unwrap(), true)
        .await
        .unwrap();
    let t = c.current_target().unwrap();
    let state = query(&mut c, &t, Query::Mappings).await.unwrap();
    println!("after restart {state}");
    assert_eq!(state["running"], true);
    assert_eq!(state["survivedRestart"], true);
}

#[tokio::test]
#[ignore]
async fn cancelled_downloads_leave_the_connection_working() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::new(identity(dir.path()));
    c.connect(std::env::var("BC_ENDPOINT").unwrap(), true)
        .await
        .unwrap();
    let t = c.current_target().unwrap();
    let id = c.status().unwrap().id;
    let path = "/data/local/tmp/bridgecoil-cancel-test";
    c.shell(&t, &format!("head -c 8000000 /dev/urandom > {path}"))
        .await
        .unwrap();
    for round in 0..40 {
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let stopped = c
            .download(&id, path, &mut Vec::new(), &cancel, |received, _| {
                if received > 200_000 {
                    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            })
            .await;
        assert_eq!(stopped.unwrap_err().kind, "cancelled", "round {round}");
        let id_now = c.status().map(|s| s.id);
        assert_eq!(id_now.as_deref(), Some(id.as_str()), "round {round}");
    }
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let mut whole = Vec::new();
    c.download(&id, path, &mut whole, &cancel, |_, _| {})
        .await
        .unwrap();
    assert_eq!(whole.len(), 8_000_000);
    c.shell(&t, &format!("rm -f {path}")).await.unwrap();
    println!("40 cancels, then a whole file");
}

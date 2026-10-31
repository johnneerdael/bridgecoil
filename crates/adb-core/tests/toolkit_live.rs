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
        engine::{Action, Workspace},
        history::History,
        queries::{Query, query},
        types::{OperationStatus, Target},
    },
};
use std::time::Duration;

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
    let mut c = Client::new(dir.path().join("identity"));
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

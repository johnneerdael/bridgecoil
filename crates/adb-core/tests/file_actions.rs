use adb_core::{
    backend::PocError,
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, Workspace},
        files::{child_path, parse_states, picked_name, valid_name},
        history::History,
        types::*,
    },
};
use async_trait::async_trait;
use std::collections::VecDeque;
struct Fake {
    target: Target,
    answers: VecDeque<String>,
    commands: Vec<String>,
}
#[async_trait]
impl ShellDriver for Fake {
    fn current_target(&self) -> Option<Target> {
        Some(self.target.clone())
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(c.into());
        Ok(ShellOutput {
            stdout: self.answers.pop_front().expect("unexpected command"),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}
fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 0,
    }
}
fn fake(answers: Vec<String>) -> Fake {
    Fake {
        target: target(),
        answers: answers.into(),
        commands: vec![],
    }
}
fn workspace(dir: &std::path::Path) -> Workspace {
    Workspace::new(History::open(&dir.join("ops.db")).unwrap())
}
fn present(path: &str, kind: &str, items: u64) -> String {
    format!("ew|{kind}|12|1700000000|{items}\0{path}\0{path}\0")
}
fn absent(path: &str) -> String {
    format!("-w|||0|0\0{path}\0{path}\0")
}
fn folder(path: &str, items: u64) -> String {
    present(path, "directory", items)
}
fn delete(path: &str) -> Action {
    Action::FileDelete { path: path.into() }
}

#[tokio::test]
async fn deleting_a_folder_says_how_much_goes_and_verifies_it_went() {
    let dir = tempfile::tempdir().unwrap();
    let path = "/sdcard/Movies/Old";
    let mut d = fake(vec![
        folder(path, 3),
        folder(path, 3),
        String::new(),
        absent(path),
    ]);
    let w = workspace(dir.path());
    let review = w.prepare(&mut d, &target(), delete(path)).await.unwrap();
    assert_eq!(review.title, "Delete Old");
    assert!(review.losses[0].contains("3 items"));
    assert!(!review.reversible);
    assert_eq!(review.commands, ["rm -rf '/sdcard/Movies/Old'"]);
    let done = w.apply(&mut d, &review.id).await.unwrap();
    assert_eq!(done.status, OperationStatus::Verified);
    assert_eq!(done.kind, "file");
}

#[tokio::test]
async fn delete_that_leaves_the_item_behind_is_not_verified() {
    let dir = tempfile::tempdir().unwrap();
    let path = "/sdcard/a.txt";
    let file = present(path, "regular file", 0);
    let mut d = fake(vec![file.clone(), file.clone(), String::new(), file]);
    let w = workspace(dir.path());
    let review = w.prepare(&mut d, &target(), delete(path)).await.unwrap();
    let done = w.apply(&mut d, &review.id).await.unwrap();
    assert_eq!(done.status, OperationStatus::Unknown);
}

#[tokio::test]
async fn storage_roots_are_protected_even_through_another_path() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    for path in ["/", "/sdcard", "/sdcard/Android", "/storage/emulated/0"] {
        let mut d = fake(vec![folder(path, 5)]);
        let e = w
            .prepare(&mut d, &target(), delete(path))
            .await
            .unwrap_err();
        assert_eq!(e.kind, "protected", "{path}");
    }
    let mut d = fake(vec![format!(
        "ew|directory|12|1|5\0/storage/self/primary/Android\0/storage/emulated/0/Android\0"
    )]);
    let e = w
        .prepare(&mut d, &target(), delete("/storage/self/primary/Android"))
        .await
        .unwrap_err();
    assert_eq!(e.kind, "protected");
}

#[tokio::test]
async fn read_only_folders_are_refused_before_any_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = "/system/app/x";
    let mut d = fake(vec![format!("e-|directory|12|1|0\0{path}\0{path}\0")]);
    let e = workspace(dir.path())
        .prepare(&mut d, &target(), delete(path))
        .await
        .unwrap_err();
    assert_eq!(e.kind, "permission");
    assert_eq!(d.commands.len(), 1);
}

#[tokio::test]
async fn a_changed_item_invalidates_its_review() {
    let dir = tempfile::tempdir().unwrap();
    let path = "/sdcard/Movies/Old";
    let mut d = fake(vec![folder(path, 3), folder(path, 4)]);
    let w = workspace(dir.path());
    let review = w.prepare(&mut d, &target(), delete(path)).await.unwrap();
    let e = w.apply(&mut d, &review.id).await.unwrap_err();
    assert_eq!(e.kind, "staleReview");
    assert_eq!(d.commands.len(), 2);
}

#[tokio::test]
async fn moving_verifies_both_ends_and_can_be_undone() {
    let dir = tempfile::tempdir().unwrap();
    let (from, to) = ("/sdcard/a.mp4", "/sdcard/Movies/a.mp4");
    let source = present(from, "regular file", 0);
    let moved = present(to, "regular file", 0);
    let before = format!("{}{}", absent(to), source);
    let after = format!("{}{}", moved, absent(from));
    let mut d = fake(vec![
        before.clone(),
        before,
        String::new(),
        after.clone(),
        after.clone(),
        after,
    ]);
    let w = workspace(dir.path());
    let action = Action::FileMove {
        from: from.into(),
        to: to.into(),
    };
    let review = w.prepare(&mut d, &target(), action).await.unwrap();
    assert_eq!(review.title, "Move a.mp4");
    assert!(review.reversible);
    assert_eq!(
        review.commands,
        ["mv -n '/sdcard/a.mp4' '/sdcard/Movies/a.mp4'"]
    );
    let done = w.apply(&mut d, &review.id).await.unwrap();
    assert_eq!(done.status, OperationStatus::Verified);
    let undo = w.prepare_recovery(&mut d, &done.id).await.unwrap();
    assert_eq!(
        undo.commands,
        ["mv -n '/sdcard/Movies/a.mp4' '/sdcard/a.mp4'"]
    );
}

#[tokio::test]
async fn a_rename_is_titled_as_one_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let (from, to) = ("/sdcard/a.txt", "/sdcard/b.txt");
    let action = Action::FileMove {
        from: from.into(),
        to: to.into(),
    };
    let w = workspace(dir.path());
    let free = format!("{}{}", present(from, "regular file", 0), absent(to));
    let mut d = fake(vec![free]);
    let review = w.prepare(&mut d, &target(), action.clone()).await.unwrap();
    assert_eq!(review.title, "Rename a.txt");
    let taken = format!(
        "{}{}",
        present(from, "regular file", 0),
        present(to, "regular file", 0)
    );
    let mut d = fake(vec![taken]);
    let e = w.prepare(&mut d, &target(), action).await.unwrap_err();
    assert_eq!(e.kind, "exists");
}

#[tokio::test]
async fn a_folder_cannot_move_into_itself() {
    let dir = tempfile::tempdir().unwrap();
    let (from, to) = ("/sdcard/a", "/sdcard/a/b/a");
    let mut d = fake(vec![format!("{}{}", folder(from, 2), absent(to))]);
    let e = workspace(dir.path())
        .prepare(
            &mut d,
            &target(),
            Action::FileMove {
                from: from.into(),
                to: to.into(),
            },
        )
        .await
        .unwrap_err();
    assert!(e.message.contains("into itself"));
}

#[tokio::test]
async fn new_folders_need_a_free_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = "/sdcard/New";
    let w = workspace(dir.path());
    let action = Action::FileCreateFolder { path: path.into() };
    let mut d = fake(vec![
        absent(path),
        absent(path),
        String::new(),
        folder(path, 0),
    ]);
    let review = w.prepare(&mut d, &target(), action.clone()).await.unwrap();
    assert_eq!(review.commands, ["mkdir '/sdcard/New'"]);
    let done = w.apply(&mut d, &review.id).await.unwrap();
    assert_eq!(done.status, OperationStatus::Verified);
    let mut d = fake(vec![folder(path, 0)]);
    let e = w.prepare(&mut d, &target(), action).await.unwrap_err();
    assert_eq!(e.kind, "exists");
}

#[tokio::test]
async fn paths_with_traversal_are_refused_rather_than_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let mut d = fake(vec![]);
    let e = workspace(dir.path())
        .prepare(&mut d, &target(), delete("/sdcard/x/../../system"))
        .await
        .unwrap_err();
    assert_eq!(e.kind, "input");
    assert!(d.commands.is_empty());
}

#[test]
fn state_output_keeps_odd_names_intact_and_rejects_nonsense() {
    let path = "/sdcard/a|b\nc";
    let states = parse_states(&present(path, "regular file", 0)).unwrap();
    assert_eq!(states[0].path, path);
    assert!(states[0].exists && states[0].parent_writable);
    assert!(parse_states("").is_err());
    assert!(parse_states("not a state").is_err());
}

#[test]
fn names_cannot_escape_their_folder() {
    assert_eq!(child_path("/", "a").unwrap(), "/a");
    assert_eq!(child_path("/sdcard/", "a b").unwrap(), "/sdcard/a b");
    for name in ["", " ", ".", "..", "a/b", "a\0b", &"x".repeat(256)] {
        assert!(!valid_name(name), "{name}");
        assert!(child_path("/sdcard", name).is_err());
    }
}

#[test]
fn picked_names_come_from_the_end_of_the_address() {
    assert_eq!(picked_name("/Users/me/Film One.mkv"), "Film One.mkv");
    assert_eq!(
        picked_name(
            "content://com.android.providers.downloads.documents/document/raw%3A%2Fstorage%2Femulated%2F0%2FDownload%2FFilm%20One.mkv"
        ),
        "Film One.mkv"
    );
    assert_eq!(
        picked_name(
            "content://com.android.externalstorage.documents/document/primary%3ADownload%2Fa.zip"
        ),
        "a.zip"
    );
    assert_eq!(picked_name("content://media/document/msf%3A1234"), "1234");
    assert_eq!(picked_name("content://x/"), "upload");
}

use super::*;
use gpui::TestAppContext;

async fn stash_fixture_command(git: &GitBinary, arguments: &[&str]) {
    let output = git.build_command(arguments).output().await.unwrap();
    assert!(
        output.status.success(),
        "{arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[gpui::test]
async fn zedx_stash_drop_removes_only_selected_record_and_apply_preserves_it(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().unwrap();
    let git = GitBinary::new(
        "git".into(),
        directory.path().to_path_buf(),
        directory.path().join(".git"),
        cx.executor(),
        false,
    );
    stash_fixture_command(&git, &["init", "-b", "main"]).await;
    for (key, value) in [
        ("user.name", "Stash fixture"),
        ("user.email", "stash@example.invalid"),
        ("commit.gpgsign", "false"),
        ("core.hooksPath", "/dev/null"),
    ] {
        stash_fixture_command(&git, &["config", "--local", key, value]).await;
    }
    let file = directory.path().join("saved.txt");
    smol::fs::write(&file, "base\n").await.unwrap();
    stash_fixture_command(&git, &["add", "saved.txt"]).await;
    stash_fixture_command(&git, &["commit", "-m", "Base"]).await;
    for value in ["first", "second", "third"] {
        smol::fs::write(&file, format!("{value}\n")).await.unwrap();
        stash_fixture_command(&git, &["stash", "push", "-m", value]).await;
    }
    let repository = RealGitRepository::new(
        &directory.path().join(".git"),
        None,
        Some("git".into()),
        cx.executor(),
    )
    .unwrap();
    let before = repository.stash_entries().await.unwrap();
    assert_eq!(before.entries.len(), 3);
    repository
        .stash_drop(Some(1), Arc::default())
        .await
        .unwrap();
    let after = repository.stash_entries().await.unwrap();
    assert_eq!(after.entries.len(), 2);
    assert_eq!(after.entries[0].oid, before.entries[0].oid);
    assert_eq!(after.entries[1].oid, before.entries[2].oid);
    assert_eq!(after.entries[1].index, 1);
    assert_eq!(smol::fs::read_to_string(&file).await.unwrap(), "base\n");
    repository
        .stash_apply(Some(1), Arc::default())
        .await
        .unwrap();
    assert_eq!(smol::fs::read_to_string(&file).await.unwrap(), "first\n");
    assert_eq!(repository.stash_entries().await.unwrap(), after);
    assert!(
        repository
            .stash_drop(Some(99), Arc::default())
            .await
            .is_err()
    );
    assert_eq!(repository.stash_entries().await.unwrap(), after);
    assert_eq!(smol::fs::read_to_string(&file).await.unwrap(), "first\n");
}

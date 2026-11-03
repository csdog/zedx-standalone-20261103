use super::*;
use gpui::TestAppContext;

async fn fixture(cx: &TestAppContext) -> (tempfile::TempDir, RealGitRepository) {
    let directory = tempfile::tempdir().unwrap();
    let git = GitBinary::new(
        "git".into(),
        directory.path().to_path_buf(),
        directory.path().join(".git"),
        cx.executor(),
        true,
    );
    git.run(&["init"]).await.unwrap();
    git.run(&["config", "core.hooksPath", "/dev/null"])
        .await
        .unwrap();
    git.run(&["config", "commit.gpgSign", "false"])
        .await
        .unwrap();
    git.run(&["config", "user.name", "Shelf test"])
        .await
        .unwrap();
    git.run(&["config", "user.email", "shelf@example.test"])
        .await
        .unwrap();
    smol::fs::write(directory.path().join("file.txt"), "base\n")
        .await
        .unwrap();
    git.run(&["add", "."]).await.unwrap();
    git.run(&["commit", "-m", "base"]).await.unwrap();
    let repository = RealGitRepository::new(
        &directory.path().join(".git"),
        None,
        Some("git".into()),
        cx.executor(),
    )
    .unwrap();
    (directory, repository)
}
#[gpui::test]
async fn shelves_round_trip_partial_staging_binary_and_untracked_without_stash(
    cx: &mut TestAppContext,
) {
    let (directory, repository) = fixture(cx).await;
    let git = repository.git_binary_in_worktree().unwrap();
    let path = directory.path().join("file.txt");
    smol::fs::write(&path, "staged\n").await.unwrap();
    git.run(&["add", "file.txt"]).await.unwrap();
    smol::fs::write(&path, "unstaged\n").await.unwrap();
    smol::fs::write(directory.path().join("binary"), [0, 255, 0, 10])
        .await
        .unwrap();
    let stash_before = repository.stash_entries().await.unwrap();
    let entry = repository.shelve("work in progress".into()).await.unwrap();
    assert_eq!(entry.file_count, 2);
    assert_eq!(smol::fs::read_to_string(&path).await.unwrap(), "base\n");
    assert!(!directory.path().join("binary").exists());
    assert!(
        git.run(&["status", "--porcelain"])
            .await
            .unwrap()
            .is_empty()
    );
    let reopened = RealGitRepository::new(
        &directory.path().join(".git"),
        None,
        Some("git".into()),
        cx.executor(),
    )
    .unwrap();
    assert_eq!(reopened.shelves().await.unwrap(), vec![entry.clone()]);
    reopened.apply_shelf(entry.id).await.unwrap();
    assert_eq!(git.run(&["show", ":file.txt"]).await.unwrap(), "staged");
    assert_eq!(smol::fs::read_to_string(&path).await.unwrap(), "unstaged\n");
    assert_eq!(
        smol::fs::read(directory.path().join("binary"))
            .await
            .unwrap(),
        [0, 255, 0, 10]
    );
    assert_eq!(
        reopened.shelves().await.unwrap().len(),
        1,
        "apply retains the shelf"
    );
    assert_eq!(repository.stash_entries().await.unwrap(), stash_before);
    reopened.delete_shelf(entry.id).await.unwrap();
    assert!(reopened.shelves().await.unwrap().is_empty());
}
#[gpui::test]
async fn conflicting_apply_preserves_index_worktree_and_shelf(cx: &mut TestAppContext) {
    let (directory, repository) = fixture(cx).await;
    let git = repository.git_binary_in_worktree().unwrap();
    let path = directory.path().join("file.txt");
    smol::fs::write(&path, "shelf\n").await.unwrap();
    let entry = repository.shelve("conflict".into()).await.unwrap();
    smol::fs::write(&path, "new work\n").await.unwrap();
    git.run(&["add", "file.txt"]).await.unwrap();
    let index = smol::fs::read(directory.path().join(".git/index"))
        .await
        .unwrap();
    assert!(repository.apply_shelf(entry.id).await.is_err());
    assert_eq!(smol::fs::read_to_string(&path).await.unwrap(), "new work\n");
    assert_eq!(
        smol::fs::read(directory.path().join(".git/index"))
            .await
            .unwrap(),
        index
    );
    assert_eq!(repository.shelves().await.unwrap(), vec![entry]);
}
#[gpui::test]
async fn shelf_respects_git_index_lock_and_empty_repository(cx: &mut TestAppContext) {
    let (directory, repository) = fixture(cx).await;
    assert!(repository.shelve("empty".into()).await.is_err());
    let lock = directory.path().join(".git/index.lock");
    smol::fs::write(&lock, b"another process").await.unwrap();
    assert!(repository.shelve("busy".into()).await.is_err());
    assert_eq!(smol::fs::read(&lock).await.unwrap(), b"another process");
    smol::fs::remove_file(lock).await.unwrap();
    assert!(repository.shelves().await.unwrap().is_empty());
}

#[gpui::test]
async fn git_log_filters_intersect_author_dates_and_literal_paths(cx: &mut TestAppContext) {
    let (directory, repository) = fixture(cx).await;
    let mut git = repository.git_binary_in_worktree().unwrap();
    for (author, date, file) in [
        ("Alice", "2026-10-01T12:00:00+0000", "alpha.txt"),
        ("Bob", "2026-10-02T12:00:00+0000", "beta.txt"),
    ] {
        smol::fs::write(directory.path().join(file), author)
            .await
            .unwrap();
        git.run(&["add", file]).await.unwrap();
        git.envs.insert("GIT_AUTHOR_NAME".into(), author.into());
        git.envs.insert("GIT_AUTHOR_DATE".into(), date.into());
        git.envs.insert("GIT_COMMITTER_DATE".into(), date.into());
        git.run(&["commit", "-m", file]).await.unwrap();
    }
    let filter = ZedxLogFilter {
        author: "Alice".into(),
        after: "2026-10-01".into(),
        before: "2026-10-01".into(),
        paths: vec!["alpha.txt".into()],
    };
    let expected = git.run(&["rev-parse", "HEAD~1"]).await.unwrap();
    assert_eq!(
        repository
            .filter_commits(LogSource::All, filter.clone())
            .await
            .unwrap(),
        vec![Oid::from_str(&expected).unwrap()]
    );
    assert!(
        repository
            .filter_commits(
                LogSource::All,
                ZedxLogFilter {
                    paths: vec!["beta.txt".into()],
                    ..filter.clone()
                }
            )
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repository
            .filter_commits(
                LogSource::All,
                ZedxLogFilter {
                    before: "2026-09-01".into(),
                    ..filter
                }
            )
            .await
            .is_err()
    );
}

#[gpui::test]
async fn revert_creates_a_commit_and_rejects_dirty_worktree(cx: &mut TestAppContext) {
    let (directory, repository) = fixture(cx).await;
    let git = repository.git_binary_in_worktree().unwrap();
    let file = directory.path().join("file.txt");
    smol::fs::write(&file, "changed\n").await.unwrap();
    git.run(&["add", "file.txt"]).await.unwrap();
    git.run(&["commit", "-m", "change"]).await.unwrap();
    let oid = Oid::from_str(&git.run(&["rev-parse", "HEAD"]).await.unwrap()).unwrap();
    repository.revert_commit(oid, Arc::default()).await.unwrap();
    assert_eq!(smol::fs::read_to_string(&file).await.unwrap(), "base\n");
    assert_ne!(
        git.run(&["rev-parse", "HEAD"]).await.unwrap(),
        oid.to_string()
    );
    smol::fs::write(&file, "keep my work\n").await.unwrap();
    assert!(repository.revert_commit(oid, Arc::default()).await.is_err());
    assert_eq!(
        smol::fs::read_to_string(&file).await.unwrap(),
        "keep my work\n"
    );
}

#[gpui::test]
async fn stash_file_count_includes_untracked_files(cx: &mut TestAppContext) {
    let (directory, repository) = fixture(cx).await;
    let git = repository.git_binary_in_worktree().unwrap();
    smol::fs::write(directory.path().join("file.txt"), "changed\n")
        .await
        .unwrap();
    smol::fs::write(directory.path().join("new.txt"), "new\n")
        .await
        .unwrap();
    git.run(&["stash", "push", "--include-untracked"])
        .await
        .unwrap();
    let stash = repository.stash_entries().await.unwrap();
    assert_eq!(
        repository
            .stash_file_count(stash.entries[0].oid)
            .await
            .unwrap(),
        2
    );
    assert_eq!(repository.stash_entries().await.unwrap(), stash);
}

async fn merge_fixture(cx: &TestAppContext) -> (tempfile::TempDir, RealGitRepository, Oid) {
    let (directory, repository) = fixture(cx).await;
    let git = repository.git_binary_in_worktree().unwrap();
    let original = git.run(&["branch", "--show-current"]).await.unwrap();
    git.run(&["switch", "-c", "shelf-test-feature"])
        .await
        .unwrap();
    smol::fs::write(directory.path().join("feature.txt"), "feature\n")
        .await
        .unwrap();
    git.run(&["add", "feature.txt"]).await.unwrap();
    git.run(&["commit", "-m", "feature"]).await.unwrap();
    git.run(&["switch", &original]).await.unwrap();
    smol::fs::write(directory.path().join("main.txt"), "main\n")
        .await
        .unwrap();
    git.run(&["add", "main.txt"]).await.unwrap();
    git.run(&["commit", "-m", "main"]).await.unwrap();
    git.run(&[
        "merge",
        "--no-ff",
        "shelf-test-feature",
        "-m",
        "merge feature",
    ])
    .await
    .unwrap();
    let oid = Oid::from_str(&git.run(&["rev-parse", "HEAD"]).await.unwrap()).unwrap();
    (directory, repository, oid)
}
#[gpui::test]
async fn merge_revert_honors_each_selected_mainline(cx: &mut TestAppContext) {
    for mainline in [1, 2] {
        let (directory, repository, oid) = merge_fixture(cx).await;
        repository
            .revert_commit_with_mainline(oid, Some(mainline), Arc::default())
            .await
            .unwrap();
        assert_eq!(directory.path().join("main.txt").exists(), mainline == 1);
        assert_eq!(directory.path().join("feature.txt").exists(), mainline == 2);
        assert_eq!(
            smol::fs::read_to_string(directory.path().join("file.txt"))
                .await
                .unwrap(),
            "base\n"
        );
        let git = repository.git_binary_in_worktree().unwrap();
        assert!(
            git.run(&["status", "--porcelain"])
                .await
                .unwrap()
                .is_empty()
        );
        assert_ne!(
            git.run(&["rev-parse", "HEAD"]).await.unwrap(),
            oid.to_string()
        );
    }
}
#[gpui::test]
async fn invalid_merge_mainline_does_not_mutate_repository(cx: &mut TestAppContext) {
    let (directory, repository, oid) = merge_fixture(cx).await;
    let git = repository.git_binary_in_worktree().unwrap();
    let index = smol::fs::read(directory.path().join(".git/index"))
        .await
        .unwrap();
    for mainline in [None, Some(0), Some(3)] {
        assert!(
            repository
                .revert_commit_with_mainline(oid, mainline, Arc::default())
                .await
                .is_err()
        );
        assert_eq!(
            git.run(&["rev-parse", "HEAD"]).await.unwrap(),
            oid.to_string()
        );
        assert_eq!(
            smol::fs::read(directory.path().join(".git/index"))
                .await
                .unwrap(),
            index
        );
        assert!(directory.path().join("main.txt").exists());
        assert!(directory.path().join("feature.txt").exists());
    }
}

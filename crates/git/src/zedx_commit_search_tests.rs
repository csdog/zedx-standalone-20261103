use super::*;
use gpui::TestAppContext;
use std::{fmt::Write as _, time::Duration};

async fn long_history(executor: BackgroundExecutor) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let git = GitBinary::new(
        PathBuf::from("git"),
        directory.path().to_path_buf(),
        directory.path().join(".git"),
        executor,
        false,
    );
    assert!(
        git.build_command(&["init", "-b", "main"])
            .output()
            .await
            .unwrap()
            .status
            .success()
    );
    let mut import = git
        .build_command(&["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut payload = String::new();
    for index in 0..20_000 {
        let subject = format!("Keep change {index:05}\n");
        write!(payload, "commit refs/heads/main\nmark :{}\ncommitter Fixture <fixture@example.invalid> {} +0000\ndata {}\n{}", index + 1, 1_700_000_000 + index, subject.len(), subject).unwrap();
        if index > 0 {
            writeln!(payload, "from :{index}").unwrap();
        }
        writeln!(payload).unwrap();
    }
    let mut input = import.stdin.take().unwrap();
    input.write_all(payload.as_bytes()).await.unwrap();
    drop(input);
    let output = import.output().await.unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    directory
}

#[gpui::test]
async fn zedx_commit_search_cancellation_releases_a_backpressured_git_process(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let directory = long_history(cx.executor()).await;
    let repository = Arc::new(
        RealGitRepository::new(
            &directory.path().join(".git"),
            None,
            Some("git".into()),
            cx.executor(),
        )
        .unwrap(),
    );
    for _ in 0..3 {
        let (sender, receiver) = async_channel::bounded(1);
        let repository = repository.clone();
        let search = cx.executor().spawn(async move {
            repository
                .search_commits(
                    LogSource::All,
                    SearchCommitArgs {
                        query: "Keep".into(),
                        case_sensitive: false,
                    },
                    sender,
                )
                .await
        });
        // Waiting for a real result proves that Git is live before cancelling.
        receiver.recv().await.unwrap();
        drop(receiver);
        // This deadline measures an external OS process, not GPUI virtual time.
        #[expect(
            clippy::disallowed_methods,
            reason = "the deadline must measure a real Git process, independently of GPUI virtual time"
        )]
        let deadline = smol::Timer::after(Duration::from_secs(10));
        match futures::future::select(search, Box::pin(deadline)).await {
            futures::future::Either::Left((result, _)) => result.unwrap(),
            futures::future::Either::Right(_) => panic!("cancelled commit search did not finish"),
        }
    }
    let (sender, receiver) = async_channel::unbounded();
    repository
        .search_commits(
            LogSource::All,
            SearchCommitArgs {
                query: "change 00000".into(),
                case_sensitive: false,
            },
            sender,
        )
        .await
        .unwrap();
    assert!(receiver.recv().await.is_ok());
    assert!(receiver.recv().await.is_err());
}

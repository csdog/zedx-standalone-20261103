async fn setup_zedx_composer(
    cx: &mut TestAppContext,
) -> (
    Arc<FakeFs>,
    Entity<Project>,
    Entity<Workspace>,
    Entity<GitPanel>,
    VisualTestContext,
) {
    init_test(cx);
    let (fs, project, workspace, panel, mut cx) = setup_git_panel_with_changes(
        cx,
        json!({".git":{},"a.txt":"first changed\n","b.txt":"second changed\n"}),
        &[
            ("a.txt", StatusCode::Modified),
            ("b.txt", StatusCode::Modified),
        ],
    )
    .await;
    register_git_commit_language(&project, &mut cx);
    let dot_git = Path::new(path!("/project/.git"));
    let sha = Oid::from_bytes(&[1; 20]).unwrap();
    fs.set_head_and_index_for_repo(
        dot_git,
        &[("a.txt", "first\n".into()), ("b.txt", "second\n".into())],
    );
    fs.set_branch_name(dot_git, Some("main"));
    fs.set_commit_data(
        dot_git,
        [(
            CommitData {
                sha,
                parents: smallvec::smallvec![],
                author_name: "Fixture".into(),
                author_email: "fixture@example.invalid".into(),
                commit_timestamp: 1,
                subject: "Previous title".into(),
                message: "Previous title\n\nPrevious body".into(),
            },
            false,
        )],
    );
    fs.with_git_state(dot_git, true, |state| {
        state.refs.insert("HEAD".into(), sha.to_string());
    })
    .unwrap();
    fs.set_graph_commits(
        dot_git,
        vec![Arc::new(InitialGraphCommitData {
            sha,
            parents: smallvec::smallvec![],
            ref_names: vec!["main".into()],
        })],
    );
    fs.set_remote_for_repo(dot_git, "origin", "https://example.invalid/origin.git");
    fs.set_remote_for_repo(dot_git, "backup", "https://example.invalid/backup.git");
    cx.run_until_parked();
    await_git_panel_entries(&panel, &mut cx).await;
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.add_panel(panel.clone(), window, cx);
        workspace.open_panel::<GitPanel>(window, cx);
    });
    cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    (fs, project, workspace, panel, cx)
}

fn click_zedx_composer(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx.debug_bounds(selector).expect(selector);
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
}

fn set_zedx_composer_draft(panel: &Entity<GitPanel>, text: &str, cx: &mut VisualTestContext) {
    panel.update_in(cx, |panel, window, cx| {
        panel
            .commit_editor
            .update(cx, |editor, cx| editor.set_text(text, window, cx))
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
}

async fn stage_zedx_composer_file(panel: &Entity<GitPanel>, cx: &mut VisualTestContext) {
    let task = panel.update(cx, |panel, cx| {
        panel
            .active_repository
            .as_ref()
            .unwrap()
            .update(cx, |repo, cx| {
                repo.stage_entries(vec![repo_path("a.txt")], cx)
            })
    });
    task.await.unwrap();
    cx.run_until_parked();
    await_git_panel_entries(panel, cx).await;
    cx.refresh().unwrap();
}

#[gpui::test]
async fn test_zedx_composer_geometry_matches_prototype_at_narrow_and_compact_sizes(
    cx: &mut TestAppContext,
) {
    let (_, _, workspace, panel, mut cx) = setup_zedx_composer(cx).await;
    set_zedx_composer_draft(&panel, "Keep this draft", &mut cx);
    for (viewport, width) in [
        (gpui::size(px(1280.0), px(900.0)), 265.0),
        (gpui::size(px(1280.0), px(900.0)), 180.0),
        (gpui::size(px(390.0), px(500.0)), 235.0),
    ] {
        cx.simulate_resize(viewport);
        workspace.update_in(&mut cx, |workspace, window, cx| {
            workspace.left_dock().update(cx, |dock, cx| {
                dock.resize_panel_sizes(Some(px(width)), None, window, cx)
            })
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        let composer = cx.debug_bounds("zedx-commit-composer").unwrap();
        let amend = cx.debug_bounds("zedx-commit-amend-row").unwrap();
        let input = cx.debug_bounds("zedx-commit-message-input").unwrap();
        let actions = cx.debug_bounds("zedx-commit-message-actions").unwrap();
        assert_eq!(amend.size.height, px(30.0));
        assert!(amend.bottom() <= input.top());
        assert_eq!(input.left() - composer.left(), px(8.0));
        assert!(input.bottom() <= actions.top());
        assert!(actions.bottom() <= composer.bottom() + px(1.0));
        let controls = [
            "zedx-commit-submit",
            "zedx-commit-push",
            "zedx-commit-options",
        ]
        .map(|id| cx.debug_bounds(id).unwrap());
        for control in controls {
            assert!(
                control.left() >= composer.left() && control.right() <= composer.right() + px(1.0)
            );
        }
        for pair in controls.windows(2) {
            assert!(pair[0].right() <= pair[1].left());
        }
        assert_eq!(controls[0].size.height, px(28.0));
    }
    panel.read_with(&cx, |panel, cx| {
        assert_eq!(panel.commit_editor.read(cx).text(cx), "Keep this draft")
    });
}

#[gpui::test]
async fn test_zedx_composer_requires_message_and_commits_only_included_files(
    cx: &mut TestAppContext,
) {
    let (fs, _, _, panel, mut cx) = setup_zedx_composer(cx).await;
    set_zedx_composer_draft(&panel, "draft", &mut cx);
    click_zedx_composer(&mut cx, "zedx-commit-message-input");
    cx.dispatch_action(git::Commit);
    cx.run_until_parked();
    cx.refresh().unwrap();
    click_zedx_composer(&mut cx, "zedx-commit-submit");
    assert_eq!(
        fs.with_git_state(Path::new(path!("/project/.git")), false, |state| state
            .commit_history
            .len())
            .unwrap(),
        0
    );
    stage_zedx_composer_file(&panel, &mut cx).await;
    set_zedx_composer_draft(&panel, "", &mut cx);
    click_zedx_composer(&mut cx, "zedx-commit-submit");
    assert!(cx.debug_bounds("zedx-commit-inspect").is_some());
    panel.update_in(&mut cx, |panel, window, cx| {
        assert!(
            panel
                .commit_editor
                .focus_handle(cx)
                .contains_focused(window, cx)
        )
    });
    cx.simulate_keystrokes("f i x");
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-commit-inspect").is_none());
    click_zedx_composer(&mut cx, "zedx-commit-submit");
    panel.read_with(&cx, |panel, cx| {
        assert!(panel.pending_commit.is_none());
        assert_eq!(panel.commit_editor.read(cx).text(cx), "");
        assert!(!panel.commit_editor.read(cx).read_only(cx));
    });
    fs.with_git_state(Path::new(path!("/project/.git")), false, |state| {
        assert_eq!(state.commit_history.len(), 1);
        assert_eq!(
            state.head_contents.get(&repo_path("a.txt")).unwrap(),
            b"first changed\n"
        );
        assert_eq!(
            state.head_contents.get(&repo_path("b.txt")).unwrap(),
            b"second\n"
        );
    })
    .unwrap();
}

#[gpui::test]
async fn test_zedx_composer_history_restores_full_message_and_amend_restores_draft(
    cx: &mut TestAppContext,
) {
    let (_, _, _, panel, mut cx) = setup_zedx_composer(cx).await;
    click_zedx_composer(&mut cx, "zedx-commit-history");
    assert!(cx.debug_bounds("zedx-commit-history-popover").is_some());
    click_zedx_composer(&mut cx, "zedx-commit-history-0");
    panel.read_with(&cx, |panel, cx| {
        assert_eq!(
            panel.commit_editor.read(cx).text(cx),
            "Previous title\n\nPrevious body"
        )
    });
    set_zedx_composer_draft(&panel, "Draft before amend", &mut cx);
    click_zedx_composer(&mut cx, "zedx-commit-amend");
    panel.read_with(&cx, |panel, cx| {
        assert!(panel.amend_pending);
        // FakeFs::show exposes its HEAD message as "initial commit".
        assert_eq!(panel.commit_editor.read(cx).text(cx), "initial commit");
    });
    click_zedx_composer(&mut cx, "zedx-commit-amend");
    panel.read_with(&cx, |panel, cx| {
        assert!(!panel.amend_pending);
        assert_eq!(panel.commit_editor.read(cx).text(cx), "Draft before amend");
    });
}

#[gpui::test]
async fn test_zedx_composer_diagnostics_and_signoff_options_are_live(cx: &mut TestAppContext) {
    let (fs, project, _, panel, mut cx) = setup_zedx_composer(cx).await;
    stage_zedx_composer_file(&panel, &mut cx).await;
    set_zedx_composer_draft(&panel, "Check code", &mut cx);
    let store = project.read_with(&cx, |project, _| project.lsp_store());
    store.update(&mut cx, |store, cx| {
        store
            .update_diagnostic_entries(
                language::LanguageServerId(0),
                path!("/project/a.txt").into(),
                None,
                None,
                vec![language::DiagnosticEntry::new(
                    Default::default()..Default::default(),
                    language::Diagnostic {
                        is_primary: true,
                        message: "fixture error".into(),
                        ..Default::default()
                    },
                )],
                cx,
            )
            .unwrap()
    });
    click_zedx_composer(&mut cx, "zedx-commit-submit");
    assert!(cx.debug_bounds("zedx-commit-inspect").is_some());
    assert_eq!(
        fs.with_git_state(Path::new(path!("/project/.git")), false, |state| state
            .commit_history
            .len())
            .unwrap(),
        0
    );
    click_zedx_composer(&mut cx, "zedx-commit-options");
    click_zedx_composer(&mut cx, "zedx-option-analysis");
    assert!(cx.debug_bounds("zedx-commit-options-popover").is_some());
    click_zedx_composer(&mut cx, "zedx-option-signoff");
    panel.read_with(&cx, |panel, _| assert!(panel.commit_options().signoff));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    cx.refresh().unwrap();
    click_zedx_composer(&mut cx, "zedx-commit-submit");
    assert_eq!(
        fs.with_git_state(Path::new(path!("/project/.git")), false, |state| state
            .commit_history
            .len())
            .unwrap(),
        1
    );
}

#[gpui::test]
async fn test_zedx_composer_commit_and_push_waits_for_commit_before_remote_choice(
    cx: &mut TestAppContext,
) {
    let (fs, _, workspace, panel, mut cx) = setup_zedx_composer(cx).await;
    stage_zedx_composer_file(&panel, &mut cx).await;
    set_zedx_composer_draft(&panel, "Commit before remote choice", &mut cx);
    click_zedx_composer(&mut cx, "zedx-commit-options");
    click_zedx_composer(&mut cx, "zedx-option-push");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    cx.refresh().unwrap();
    click_zedx_composer(&mut cx, "zedx-commit-submit");
    assert_eq!(
        fs.with_git_state(Path::new(path!("/project/.git")), false, |state| state
            .commit_history
            .len())
            .unwrap(),
        1
    );
    workspace.read_with(&cx, |workspace, cx| {
        assert!(
            workspace
                .active_modal::<crate::picker_prompt::PickerPrompt>(cx)
                .is_some()
        )
    });
    panel.read_with(&cx, |panel, cx| {
        assert!(panel.pending_commit.is_none());
        assert!(!panel.commit_editor.read(cx).read_only(cx));
    });
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.hide_modal(window, cx)
    });
    cx.run_until_parked();
}

#[gpui::test]
async fn test_zedx_composer_failed_commit_retains_draft_and_does_not_offer_push(
    cx: &mut TestAppContext,
) {
    let (fs, _, workspace, panel, mut cx) = setup_zedx_composer(cx).await;
    stage_zedx_composer_file(&panel, &mut cx).await;
    fs.with_git_state(Path::new(path!("/project/.git")), false, |state| {
        state.index_contents = state.head_contents.clone()
    })
    .unwrap();
    set_zedx_composer_draft(&panel, "Keep failed draft", &mut cx);
    click_zedx_composer(&mut cx, "zedx-commit-push");
    panel.read_with(&cx, |panel, cx| {
        assert!(panel.pending_commit.is_none());
        assert!(!panel.commit_editor.read(cx).read_only(cx));
        assert_eq!(panel.commit_editor.read(cx).text(cx), "Keep failed draft");
    });
    assert_eq!(
        fs.with_git_state(Path::new(path!("/project/.git")), false, |state| state
            .commit_history
            .len())
            .unwrap(),
        0
    );
    workspace.read_with(&cx, |workspace, cx| {
        assert!(
            workspace
                .active_modal::<crate::picker_prompt::PickerPrompt>(cx)
                .is_none()
        )
    });
}

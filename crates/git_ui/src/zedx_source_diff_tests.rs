async fn setup_zedx_source_diff(
    cx: &mut TestAppContext,
) -> (Entity<Workspace>, Entity<GitPanel>, VisualTestContext) {
    init_test(cx);
    cx.update(|cx| {
        cx.bind_keys([gpui::KeyBinding::new(
            "ctrl-d",
            editor::actions::Delete,
            Some("Editor"),
        )]);
        crate::bind_zedx_source_diff_keys(cx);
    });
    let fs = FakeFs::new(cx.background_executor.clone());
    fs.insert_tree(path!("/project"), json!({
        ".git": {}, "a.txt": "first changed\n", "b.txt": "second changed\n", "clean.txt": "clean\n"
    })).await;
    fs.set_head_and_index_for_repo(
        path!("/project/.git").as_ref(),
        &[
            ("a.txt", "first\n".into()),
            ("b.txt", "second\n".into()),
            ("clean.txt", "clean\n".into()),
        ],
    );
    let project = Project::test(fs, [Path::new(path!("/project"))], cx).await;
    let window_handle =
        cx.add_window(|window, cx| MultiWorkspace::test_new(project.clone(), window, cx));
    let workspace = window_handle
        .read_with(cx, |mw, _| mw.workspace().clone())
        .unwrap();
    let mut cx = VisualTestContext::from_window(window_handle.into(), cx);
    cx.read(|cx| {
        project
            .read(cx)
            .worktrees(cx)
            .next()
            .unwrap()
            .read(cx)
            .as_local()
            .unwrap()
            .scan_complete()
    })
    .await;
    cx.run_until_parked();
    let panel = workspace.update_in(&mut cx, GitPanel::new);
    await_git_panel_entries(&panel, &mut cx).await;
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.add_panel(panel.clone(), window, cx)
    });
    (workspace, panel, cx)
}

async fn open_zedx_source(
    name: &str,
    workspace: &Entity<Workspace>,
    panel: &Entity<GitPanel>,
    cx: &mut VisualTestContext,
) -> Entity<Editor> {
    let path = panel.read_with(cx, |panel, cx| {
        panel
            .active_repository
            .as_ref()
            .unwrap()
            .read(cx)
            .repo_path_to_project_path(&repo_path(name), cx)
            .unwrap()
    });
    workspace
        .update_in(cx, |workspace, window, cx| {
            workspace.open_path(path, None, true, window, cx)
        })
        .await
        .unwrap();
    cx.run_until_parked();
    cx.refresh().unwrap();
    workspace.read_with(cx, |workspace, cx| {
        workspace.active_item_as::<Editor>(cx).unwrap()
    })
}

#[gpui::test]
async fn test_zedx_source_diff_control_d_round_trip_preserves_text_cursor_and_tabs(
    cx: &mut TestAppContext,
) {
    let (workspace, panel, mut cx) = setup_zedx_source_diff(cx).await;
    let source = open_zedx_source("a.txt", &workspace, &panel, &mut cx).await;
    source.update_in(&mut cx, |editor, window, cx| {
        editor.change_selections(
            editor::SelectionEffects::no_scroll(),
            window,
            cx,
            |selections| {
                selections.select_ranges([language::Point::new(0, 3)..language::Point::new(0, 3)]);
            },
        );
    });
    for _ in 0..3 {
        cx.simulate_keystrokes("ctrl-d");
        cx.run_until_parked();
        cx.refresh().unwrap();
        workspace.read_with(&cx, |workspace, cx| {
            let diff = workspace
                .active_item_as::<SoloDiffView>(cx)
                .expect("Control+D opens the file diff");
            assert_eq!(
                diff.project_path(cx).unwrap().path.as_ref(),
                rel_path("a.txt")
            );
            assert_eq!(workspace.items_of_type::<SoloDiffView>(cx).count(), 1);
            assert_eq!(workspace.items_of_type::<ProjectDiff>(cx).count(), 0);
        });
        cx.simulate_keystrokes("ctrl-d");
        cx.run_until_parked();
        cx.refresh().unwrap();
        workspace.read_with(&cx, |workspace, cx| {
            assert_eq!(workspace.active_item_as::<Editor>(cx), Some(source.clone()));
            assert_eq!(workspace.items_of_type::<Editor>(cx).count(), 1);
            let editor = source.read(cx);
            assert_eq!(
                editor.text(cx),
                "first changed\n",
                "Control+D must not delete source text"
            );
            let snapshot = editor.buffer().read(cx).snapshot(cx);
            assert_eq!(
                multi_buffer::ToPoint::to_point(
                    &editor.selections.newest_anchor().head(),
                    &snapshot
                ),
                language::Point::new(0, 3)
            );
        });
    }
    open_zedx_source("b.txt", &workspace, &panel, &mut cx).await;
    cx.simulate_keystrokes("ctrl-d");
    cx.run_until_parked();
    workspace.read_with(&cx, |workspace, cx| {
        let diff = workspace.active_item_as::<SoloDiffView>(cx).unwrap();
        assert_eq!(
            diff.project_path(cx).unwrap().path.as_ref(),
            rel_path("b.txt")
        );
        assert_eq!(workspace.items_of_type::<SoloDiffView>(cx).count(), 2);
    });
}

#[gpui::test]
async fn test_zedx_source_diff_control_d_scopes_out_commit_message(cx: &mut TestAppContext) {
    let (workspace, panel, mut cx) = setup_zedx_source_diff(cx).await;
    let source = open_zedx_source("clean.txt", &workspace, &panel, &mut cx).await;
    cx.simulate_keystrokes("ctrl-d");
    cx.run_until_parked();
    workspace.read_with(&cx, |workspace, cx| {
        assert_eq!(workspace.active_item_as::<Editor>(cx), Some(source.clone()));
        assert_eq!(source.read(cx).text(cx), "clean\n");
        assert_eq!(workspace.items_of_type::<SoloDiffView>(cx).count(), 0);
    });
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.focus_panel::<GitPanel>(window, cx)
    });
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.commit_editor.update(cx, |editor, cx| {
            editor.set_text("draft", window, cx);
            editor.change_selections(
                editor::SelectionEffects::no_scroll(),
                window,
                cx,
                |selections| {
                    selections.select_ranges([language::Point::zero()..language::Point::zero()]);
                },
            );
            editor.focus_handle(cx).focus(window, cx);
        });
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.simulate_keystrokes("ctrl-d");
    cx.run_until_parked();
    panel.read_with(&cx, |panel, cx| {
        assert_eq!(
            panel.commit_editor.read(cx).text(cx),
            "raft",
            "commit message retains the editor's normal Control+D behavior"
        )
    });
    workspace.read_with(&cx, |workspace, cx| {
        assert!(workspace.active_item_as::<SoloDiffView>(cx).is_none())
    });
}

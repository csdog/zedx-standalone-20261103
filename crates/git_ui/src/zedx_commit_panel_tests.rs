#[gpui::test]
async fn test_zedx_commit_click_and_confirm_open_only_target_file(cx: &mut TestAppContext) {
    init_test(cx);
    let (_, _, workspace, panel, mut cx) = setup_git_panel_with_changes(
        cx,
        json!({ ".git": {}, "a.txt": "a changed\n", "b.txt": "b changed\n" }),
        &[
            ("a.txt", StatusCode::Modified),
            ("b.txt", StatusCode::Modified),
        ],
    )
    .await;
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.add_panel(panel.clone(), window, cx);
        workspace.open_panel::<GitPanel>(window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let bounds = cx
        .debug_bounds("zedx-commit-file-a.txt")
        .expect("first file row");
    cx.simulate_click(
        gpui::point(bounds.left() + px(51.0), bounds.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    workspace.read_with(&cx, |workspace, cx| {
        assert_eq!(
            workspace.items_of_type::<SoloDiffView>(cx).count(),
            0,
            "staging checkbox must not open a diff"
        );
    });
    await_git_panel_entries(&panel, &mut cx).await;
    panel.read_with(&cx, |panel, cx| {
        let repository = panel.active_repository.as_ref().unwrap().read(cx);
        let entry = panel
            .change_entries_by_path()
            .find(|entry| entry.repo_path == repo_path("a.txt"))
            .unwrap();
        assert_eq!(
            GitPanel::stage_status_for_entry(entry, repository),
            StageStatus::Staged
        );
    });
    cx.refresh().unwrap();
    let bounds = cx.debug_bounds("zedx-commit-file-a.txt").unwrap();
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(&cx, |workspace, cx| {
        let diff = workspace
            .active_item_as::<SoloDiffView>(cx)
            .expect("single file diff");
        assert_eq!(
            diff.project_path(cx).unwrap().path.as_ref(),
            rel_path("a.txt")
        );
        assert_eq!(workspace.items_of_type::<SoloDiffView>(cx).count(), 1);
        assert_eq!(workspace.items_of_type::<ProjectDiff>(cx).count(), 0);
    });
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.selected_entry = entry_index_for_repo_path(panel, &repo_path("b.txt"));
        mark_file(panel, "a.txt");
        mark_file(panel, "b.txt");
        panel.confirm_panel_entry(&menu::Confirm, window, cx);
    });
    cx.run_until_parked();
    workspace.read_with(&cx, |workspace, cx| {
        let diff = workspace
            .active_item_as::<SoloDiffView>(cx)
            .expect("second file diff");
        assert_eq!(
            diff.project_path(cx).unwrap().path.as_ref(),
            rel_path("b.txt")
        );
        assert_eq!(workspace.items_of_type::<SoloDiffView>(cx).count(), 2);
        assert_eq!(workspace.items_of_type::<ProjectDiff>(cx).count(), 0);
    });
    cx.refresh().unwrap();
    let bounds = cx.debug_bounds("zedx-commit-file-a.txt").unwrap();
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(&cx, |workspace, cx| {
        let diff = workspace.active_item_as::<SoloDiffView>(cx).unwrap();
        assert_eq!(
            diff.project_path(cx).unwrap().path.as_ref(),
            rel_path("a.txt")
        );
        assert_eq!(
            workspace.items_of_type::<SoloDiffView>(cx).count(),
            2,
            "reuse existing file tab"
        );
    });
}

#[gpui::test]
async fn test_zedx_commit_tabs_keep_draft_and_match_header_geometry(cx: &mut TestAppContext) {
    init_test(cx);
    let (_, _, workspace, panel, mut cx) = setup_git_panel_with_changes(
        cx,
        json!({ ".git": {}, "a.txt": "changed\n" }),
        &[("a.txt", StatusCode::Modified)],
    )
    .await;
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.add_panel(panel.clone(), window, cx);
        workspace.open_panel::<GitPanel>(window, cx);
    });
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.commit_editor.update(cx, |editor, cx| {
            editor.set_text("Keep my draft", window, cx)
        });
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let tabs = cx.debug_bounds("zedx-commit-tabs").unwrap();
    let toolbar = cx.debug_bounds("zedx-commit-toolbar").unwrap();
    assert_eq!(tabs.size.height, px(35.0));
    assert_eq!(toolbar.size.height, px(30.0));
    assert_eq!(toolbar.top(), tabs.bottom());
    let commit = cx.debug_bounds("zedx-commit-tab-Commit").unwrap();
    let stash = cx.debug_bounds("zedx-commit-tab-Stash").unwrap();
    let shelf = cx.debug_bounds("zedx-commit-tab-Shelf").unwrap();
    assert!(commit.right() <= stash.left() && stash.right() <= shelf.left());
    assert!(shelf.right() <= tabs.right());
    cx.simulate_click(stash.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-commit-file-a.txt").is_none());
    assert!(cx.debug_bounds("zedx-commit-toolbar").is_none());
    let shelf = cx.debug_bounds("zedx-commit-tab-Shelf").unwrap();
    cx.simulate_click(shelf.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    let commit = cx.debug_bounds("zedx-commit-tab-Commit").unwrap();
    cx.simulate_click(commit.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-commit-file-a.txt").is_some());
    panel.read_with(&cx, |panel, cx| {
        assert_eq!(panel.commit_editor.read(cx).text(cx), "Keep my draft")
    });
}

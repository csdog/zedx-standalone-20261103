#[gpui::test]
async fn shelf_opens_real_list_and_reports_backend_capability_without_losing_draft(
    cx: &mut TestAppContext,
) {
    let (_, workspace, panel, mut cx) = setup_zedx_stashes(cx).await;
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.commit_editor.update(cx, |editor, cx| {
            editor.set_text("Retain this draft", window, cx)
        });
        panel.select_zedx_commit_tab(super::zedx_commit_panel::CommitTab::Shelf, window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-shelf-list").is_some());
    // The fake Git backend has no shelf capability. Its explicit error must be shown.
    assert!(cx.debug_bounds("zedx-shelf-error").is_some());
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.select_zedx_commit_tab(super::zedx_commit_panel::CommitTab::Commit, window, cx);
    });
    panel.read_with(&cx, |panel, cx| {
        assert_eq!(panel.commit_editor.read(cx).text(cx), "Retain this draft")
    });
    workspace.read_with(&cx, |workspace, cx| {
        assert_eq!(
            workspace.project().read(cx).visible_worktrees(cx).count(),
            1
        )
    });
}

fn keyboard_shelves() -> Vec<git::repository::ShelfEntry> {
    (1..=3)
        .map(|index| git::repository::ShelfEntry {
            id: format!("00000000-0000-0000-0000-{index:012}")
                .parse()
                .unwrap(),
            message: format!("Shelf {index}"),
            timestamp: 1_700_000_000,
            file_count: index,
        })
        .collect()
}
#[gpui::test]
async fn shelf_keyboard_wraps_and_reaches_first_and_last(cx: &mut TestAppContext) {
    let (_, _, panel, mut cx) = setup_zedx_stashes(cx).await;
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.select_zedx_commit_tab(super::zedx_commit_panel::CommitTab::Shelf, window, cx);
        panel.set_zedx_shelves_for_test(keyboard_shelves(), cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.simulate_keystrokes("down");
    panel.read_with(&cx, |panel, _| {
        assert_eq!(
            panel.zedx_selected_shelf_for_test().unwrap().message,
            "Shelf 1"
        )
    });
    cx.simulate_keystrokes("up");
    panel.read_with(&cx, |panel, _| {
        assert_eq!(
            panel.zedx_selected_shelf_for_test().unwrap().message,
            "Shelf 3"
        )
    });
    cx.dispatch_action(menu::SelectFirst);
    panel.read_with(&cx, |panel, _| {
        assert_eq!(
            panel.zedx_selected_shelf_for_test().unwrap().message,
            "Shelf 1"
        )
    });
    cx.dispatch_action(menu::SelectLast);
    panel.read_with(&cx, |panel, _| {
        assert_eq!(
            panel.zedx_selected_shelf_for_test().unwrap().message,
            "Shelf 3"
        )
    });
}
#[gpui::test]
async fn shelf_enter_and_delete_require_confirmation_and_cancel_preserves_selection(
    cx: &mut TestAppContext,
) {
    let (_, _, panel, mut cx) = setup_zedx_stashes(cx).await;
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.select_zedx_commit_tab(super::zedx_commit_panel::CommitTab::Shelf, window, cx);
        panel.set_zedx_shelves_for_test(keyboard_shelves(), cx);
        panel.select_zedx_shelf(false, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    for key in ["enter", "delete"] {
        cx.simulate_keystrokes(key);
        cx.run_until_parked();
        assert!(cx.has_pending_prompt(), "{key} must ask before a mutation");
        cx.simulate_prompt_answer("Cancel");
        cx.run_until_parked();
        cx.refresh().unwrap();
        panel.read_with(&cx, |panel, _| {
            assert_eq!(
                panel.zedx_selected_shelf_for_test().unwrap().message,
                "Shelf 1"
            )
        });
    }
}

#[gpui::test]
async fn stale_shelf_selection_is_cleared_before_prompting_for_another_repository(
    cx: &mut TestAppContext,
) {
    let (_, _, panel, mut cx) = setup_zedx_stashes(cx).await;
    let other = panel.read_with(&cx, |panel, cx| {
        panel
            .project
            .read(cx)
            .git_store()
            .read(cx)
            .repositories()
            .values()
            .find(|repository| {
                repository.read(cx).work_directory_abs_path.as_ref() == Path::new(path!("/other"))
            })
            .unwrap()
            .clone()
    });
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.select_zedx_commit_tab(super::zedx_commit_panel::CommitTab::Shelf, window, cx);
        panel.set_zedx_shelves_for_test(keyboard_shelves(), cx);
        panel.select_zedx_shelf(false, cx);
        // Exercise the gap between a repository update and the next render.
        panel.active_repository = Some(other);
        panel.confirm_panel_entry(&menu::Confirm, window, cx);
        assert!(panel.zedx_selected_shelf_for_test().is_none());
    });
    assert!(!cx.has_pending_prompt());
}

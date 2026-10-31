use super::*;
use gpui::TestAppContext;

#[gpui::test]
async fn find_searches_real_project_and_enter_navigates_selected_match(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    cx.dispatch_action(FindInFiles {
        query: Some("search fixture".into()),
        include_ignored: None,
    });
    cx.run_until_parked();
    let find = workspace.read_with(cx, |workspace, cx| workspace.panel::<Find>(cx).unwrap());
    find.read_with(cx, |find, cx| {
        assert_eq!(find.rows.len(), 2);
        assert_eq!(
            find.entries.len(),
            4,
            "each file has a header and a matching line"
        );
        for row in &find.rows {
            assert_eq!(&row.text[row.highlight.clone()], "search fixture");
        }
        assert_eq!(find.badge(cx), Some("2".into()));
        assert!(!find.searching);
        assert!(find.error.is_none());
    });
    let selected = find.read_with(cx, |find, _| find.rows[1].clone());
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        let editor = workspace.active_item_as::<Editor>(cx).unwrap();
        assert!(editor.focus_handle(cx).contains_focused(window, cx));
        editor.update(cx, |editor, cx| {
            let snapshot = editor
                .display_map
                .update(cx, |display, cx| display.snapshot(cx));
            let selection = editor.selections.newest::<language::Point>(&snapshot);
            let buffer_snapshot = selected.buffer.read(cx).snapshot();
            assert_eq!(
                selection.start,
                buffer_snapshot.summary_for_anchor::<language::Point>(&selected.range.start)
            );
            assert_eq!(
                selection.end,
                buffer_snapshot.summary_for_anchor::<language::Point>(&selected.range.end)
            );
        });
    });
}

#[gpui::test]
async fn find_filters_ignored_files_and_cancels_obsolete_queries(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    cx.dispatch_action(FindInFiles {
        query: Some("externalneedle".into()),
        include_ignored: Some(false),
    });
    cx.run_until_parked();
    let find = workspace.read_with(cx, |workspace, cx| workspace.panel::<Find>(cx).unwrap());
    assert!(find.read_with(cx, |find, _| find.rows.is_empty()));
    cx.dispatch_action(FindInFiles {
        query: Some("externalneedle".into()),
        include_ignored: Some(true),
    });
    cx.run_until_parked();
    assert_eq!(find.read_with(cx, |find, _| find.rows.len()), 1);
    find.update_in(cx, |find, window, cx| {
        find.configure(
            &FindInFiles {
                query: Some("fixture".into()),
                include_ignored: None,
            },
            window,
            cx,
        );
        find.configure(
            &FindInFiles {
                query: Some("missing_xyz_982374".into()),
                include_ignored: None,
            },
            window,
            cx,
        );
    });
    cx.run_until_parked();
    find.read_with(cx, |find, _| {
        assert!(find.rows.is_empty());
        assert!(!find.searching);
    });
    cx.simulate_keystrokes("cmd-shift-f");
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        assert!(
            find.read(cx)
                .query
                .focus_handle(cx)
                .contains_focused(window, cx)
        );
        assert_eq!(
            workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .active_panel()
                .unwrap()
                .persistent_name(),
            "Find"
        );
    });
}

#[gpui::test]
async fn find_groups_unicode_matches_and_highlights_the_actual_source_ranges(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let fs = workspace.read_with(cx, |workspace, cx| {
        workspace.project().read(cx).fs().clone()
    });
    fs.as_fake()
        .insert_tree(
            util::path!("/project"),
            serde_json::json!({"unicode.rs": "    // 主题 主题\n"}),
        )
        .await;
    cx.run_until_parked();
    cx.dispatch_action(FindInFiles {
        query: Some("主题".into()),
        include_ignored: None,
    });
    cx.run_until_parked();
    let find = workspace.read_with(cx, |workspace, cx| workspace.panel::<Find>(cx).unwrap());
    find.read_with(cx, |find, _| {
        assert_eq!(find.rows.len(), 2);
        assert_eq!(find.entries.len(), 3);
        assert!(
            matches!(&find.entries[0], Entry::File {path, count: 2} if path.ends_with("unicode.rs"))
        );
        for row in &find.rows {
            assert!(row.text.is_char_boundary(row.highlight.start));
            assert!(row.text.is_char_boundary(row.highlight.end));
            assert_eq!(&row.text[row.highlight.clone()], "主题");
        }
        assert_ne!(find.rows[0].highlight, find.rows[1].highlight);
    });
}

#[gpui::test]
async fn clearing_find_query_restores_typing_focus_and_searches_again(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    cx.dispatch_action(FindInFiles {
        query: Some("fixture".into()),
        include_ignored: None,
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let clear = cx.debug_bounds("zedx-find-clear-button").unwrap();
    cx.simulate_click(clear.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    let find = workspace.read_with(cx, |workspace, cx| workspace.panel::<Find>(cx).unwrap());
    find.update_in(cx, |find, window, cx| {
        assert!(find.rows.is_empty());
        assert!(find.query.read(cx).text(cx).is_empty());
        assert!(find.query.focus_handle(cx).contains_focused(window, cx));
    });
    cx.simulate_input("fixture");
    cx.run_until_parked();
    assert_eq!(find.read_with(cx, |find, _| find.rows.len()), 2);
}

use super::*;
use gpui::{TestAppContext, VisualTestContext};

async fn open_file(workspace: &Entity<Workspace>, name: &str, cx: &mut VisualTestContext) {
    let path = workspace.read_with(cx, |workspace, cx| ProjectPath {
        worktree_id: workspace
            .project()
            .read(cx)
            .visible_worktrees(cx)
            .next()
            .unwrap()
            .read(cx)
            .id(),
        path: RelPath::from_unix_str(name).unwrap().into(),
    });
    workspace
        .update_in(cx, |workspace, window, cx| {
            workspace.open_path(path, None, true, window, cx)
        })
        .await
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
async fn ctrl_e_opens_recent_files_and_cycles_without_recreating_the_popup(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = super::super::tests::setup(cx).await;
    open_file(&workspace, "main.rs", cx).await;
    open_file(&workspace, "README.md", cx).await;
    cx.simulate_keystrokes("ctrl-e");
    cx.run_until_parked();
    let modal = workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<RecentWindow>(cx).unwrap()
    });
    modal.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert_eq!(delegate.rows.len(), 2);
        assert_eq!(delegate.rows[delegate.selected].label, "main.rs");
    });
    cx.simulate_keystrokes("ctrl-e");
    cx.run_until_parked();
    let repeated = workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<RecentWindow>(cx).unwrap()
    });
    assert_eq!(modal.entity_id(), repeated.entity_id());
    repeated.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert_eq!(delegate.rows[delegate.selected].label, "README.md");
    });
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(workspace.active_modal::<RecentWindow>(cx).is_none());
        assert_eq!(
            workspace
                .active_item(cx)
                .unwrap()
                .project_path(cx)
                .unwrap()
                .path
                .as_unix_str(),
            "main.rs"
        );
    });
}

#[gpui::test]
async fn recent_files_filter_stays_inside_history_and_escape_restores_editor_focus(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = super::super::tests::setup(cx).await;
    open_file(&workspace, "main.rs", cx).await;
    let focus = workspace.read_with(cx, |workspace, cx| {
        workspace.active_item(cx).unwrap().item_focus_handle(cx)
    });
    cx.simulate_keystrokes("ctrl-e");
    cx.run_until_parked();
    let modal = workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<RecentWindow>(cx).unwrap()
    });
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("README", window, cx))
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        0
    );
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("MAIN", window, cx))
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        1
    );
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        assert!(workspace.active_modal::<RecentWindow>(cx).is_none());
        assert!(focus.contains_focused(window, cx));
    });
}

#[gpui::test]
async fn recent_files_empty_state_fits_desktop_and_compact_windows(cx: &mut TestAppContext) {
    let (workspace, cx) = super::super::tests::setup(cx).await;
    cx.simulate_keystrokes("ctrl-e");
    cx.run_until_parked();
    let modal = workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<RecentWindow>(cx).unwrap()
    });
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        0
    );
    for viewport in [
        gpui::size(px(1280.0), px(900.0)),
        gpui::size(px(390.0), px(500.0)),
    ] {
        cx.simulate_resize(viewport);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let bounds = cx.debug_bounds("zedx-recent-files-modal").unwrap();
        assert!(bounds.left() >= px(0.0));
        assert!(bounds.right() <= viewport.width);
        assert!(bounds.bottom() <= viewport.height);
    }
}

#[gpui::test]
async fn recent_files_show_quick_navigation_and_allow_keyboard_switching(cx: &mut TestAppContext) {
    let (workspace, cx) = super::super::tests::setup(cx).await;
    open_file(&workspace, "main.rs", cx).await;
    cx.simulate_keystrokes("ctrl-e");
    cx.run_until_parked();
    let modal = workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<RecentWindow>(cx).unwrap()
    });
    for viewport in [
        gpui::size(px(1280.0), px(900.0)),
        gpui::size(px(390.0), px(500.0)),
    ] {
        cx.simulate_resize(viewport);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let navigation = cx.debug_bounds("zedx-recent-quick-navigation").unwrap();
        let files = cx.debug_bounds("zedx-recent-file-list").unwrap();
        assert!(navigation.right() <= files.left());
        assert!(files.right() <= viewport.width);
    }
    cx.simulate_keystrokes("left");
    cx.run_until_parked();
    modal.update_in(cx, |modal, window, cx| {
        assert!(modal.navigation_focus.contains_focused(window, cx))
    });
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
    assert_eq!(modal.read_with(cx, |modal, _| modal.navigation_selected), 1);
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    modal.update_in(cx, |modal, window, cx| {
        assert!(modal.picker.focus_handle(cx).contains_focused(window, cx))
    });
}

#[gpui::test]
async fn quick_navigation_dismisses_recent_files_before_opening_settings(cx: &mut TestAppContext) {
    let (workspace, cx) = super::super::tests::setup(cx).await;
    let opened = std::rc::Rc::new(std::cell::Cell::new(false));
    cx.update({
        let opened = opened.clone();
        move |_, cx| cx.on_action(move |_: &zed_actions::OpenSettings, _| opened.set(true))
    });
    cx.simulate_keystrokes("ctrl-e");
    cx.run_until_parked();
    let modal = workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<RecentWindow>(cx).unwrap()
    });
    modal.update_in(cx, |modal, window, cx| modal.navigate(8, window, cx));
    cx.run_until_parked();
    assert!(opened.get());
    workspace.read_with(cx, |workspace, cx| {
        assert!(workspace.active_modal::<RecentWindow>(cx).is_none())
    });
}

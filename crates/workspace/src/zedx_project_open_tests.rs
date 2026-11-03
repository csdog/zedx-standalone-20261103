use super::*;
use fs::FakeFs;
use gpui::{TestAppContext, VisualTestContext};
use serde_json::json;
use std::path::Path;

async fn setup(
    cx: &mut TestAppContext,
) -> (Entity<MultiWorkspace>, Entity<Workspace>, VisualTestContext) {
    crate::tests::init_test(cx);
    let fs = FakeFs::new(cx.executor());
    cx.update(|cx| <dyn fs::Fs>::set_global(fs.clone(), cx));
    fs.insert_tree("/first", json!({"old.txt":"old"})).await;
    fs.insert_tree("/second", json!({"new.txt":"new"})).await;
    let project = Project::test(fs, [Path::new("/first")], cx).await;
    let handle = cx.add_window(|window, cx| MultiWorkspace::test_new(project, window, cx));
    let multi = handle.update(cx, |_, _, cx| cx.entity()).unwrap();
    let workspace = handle
        .read_with(cx, |multi, _| multi.workspace().clone())
        .unwrap();
    let visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    (multi, workspace, visual)
}

fn roots(workspace: &Entity<Workspace>, cx: &App) -> Vec<PathBuf> {
    workspace
        .read(cx)
        .project()
        .read(cx)
        .visible_worktrees(cx)
        .map(|worktree| worktree.read(cx).abs_path().to_path_buf())
        .collect()
}

#[gpui::test]
async fn zedx_open_project_attach_keeps_current_workspace_and_adds_root(cx: &mut TestAppContext) {
    let (multi, original, mut cx) = setup(cx).await;
    let task = multi.update_in(&mut cx, |multi, window, cx| {
        multi.open_project_with_zedx_prompt(vec!["/second".into()], false, window, cx)
    });
    cx.run_until_parked();
    assert!(cx.pending_prompt().unwrap().1.contains("/second"));
    cx.simulate_prompt_answer("Attach");
    task.await.unwrap();
    multi.read_with(&cx, |multi, cx| {
        assert_eq!(multi.workspace(), &original);
        assert_eq!(
            roots(&original, cx),
            [PathBuf::from("/first"), PathBuf::from("/second")]
        );
    });
}

#[gpui::test]
async fn zedx_open_project_this_window_replaces_current_project(cx: &mut TestAppContext) {
    let (multi, original, mut cx) = setup(cx).await;
    let task = multi.update_in(&mut cx, |multi, window, cx| {
        multi.open_project_with_zedx_prompt(vec!["/second".into()], false, window, cx)
    });
    cx.run_until_parked();
    cx.simulate_prompt_answer("This Window");
    task.await.unwrap();
    multi.read_with(&cx, |multi, cx| {
        assert_ne!(multi.workspace(), &original);
        assert_eq!(roots(multi.workspace(), cx), [PathBuf::from("/second")]);
        assert!(!multi.workspaces().any(|workspace| *workspace == original));
    });
    assert_eq!(cx.update(|_, cx| cx.windows().len()), 1);
}

#[gpui::test]
async fn zedx_open_project_new_window_keeps_current_project(cx: &mut TestAppContext) {
    let (multi, original, mut cx) = setup(cx).await;
    let task = multi.update_in(&mut cx, |multi, window, cx| {
        multi.open_project_with_zedx_prompt(vec!["/second".into()], false, window, cx)
    });
    cx.run_until_parked();
    cx.simulate_prompt_answer("New Window");
    task.await.unwrap();
    multi.read_with(&cx, |multi, cx| {
        assert_eq!(multi.workspace(), &original);
        assert_eq!(roots(&original, cx), [PathBuf::from("/first")]);
    });
    assert_eq!(cx.update(|_, cx| cx.windows().len()), 2);
}

#[gpui::test]
async fn zedx_open_project_cancel_changes_nothing(cx: &mut TestAppContext) {
    let (multi, original, mut cx) = setup(cx).await;
    let task = multi.update_in(&mut cx, |multi, window, cx| {
        multi.open_project_with_zedx_prompt(vec!["/second".into()], true, window, cx)
    });
    cx.run_until_parked();
    cx.simulate_prompt_answer("Cancel");
    task.await.unwrap();
    multi.read_with(&cx, |multi, cx| {
        assert_eq!(multi.workspace(), &original);
        assert_eq!(roots(&original, cx), [PathBuf::from("/first")]);
    });
    assert_eq!(cx.update(|_, cx| cx.windows().len()), 1);
}

#[gpui::test]
async fn zedx_open_project_replace_honors_cancelled_save(cx: &mut TestAppContext) {
    let (multi, original, mut cx) = setup(cx).await;
    let item = cx.new(|cx| crate::item::test::TestItem::new(cx).with_dirty(true));
    original.update_in(&mut cx, |workspace, window, cx| {
        workspace.add_item_to_active_pane(Box::new(item.clone()), None, true, window, cx)
    });
    let task = multi.update_in(&mut cx, |multi, window, cx| {
        multi.open_project_with_zedx_prompt(vec!["/second".into()], false, window, cx)
    });
    cx.run_until_parked();
    cx.simulate_prompt_answer("This Window");
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Cancel");
    task.await.unwrap();
    multi.read_with(&cx, |multi, cx| {
        assert_eq!(multi.workspace(), &original);
        assert_eq!(roots(&original, cx), [PathBuf::from("/first")]);
    });
}

#[gpui::test]
async fn zedx_open_project_same_folder_preserves_dirty_items_without_save_prompt(
    cx: &mut TestAppContext,
) {
    let (multi, original, mut cx) = setup(cx).await;
    let item = cx.new(|cx| crate::item::test::TestItem::new(cx).with_dirty(true));
    original.update_in(&mut cx, |workspace, window, cx| {
        workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx)
    });
    let task = multi.update_in(&mut cx, |multi, window, cx| {
        multi.open_project_with_zedx_prompt(vec!["/first".into()], false, window, cx)
    });
    cx.run_until_parked();
    cx.simulate_prompt_answer("This Window");
    task.await.unwrap();
    assert!(!cx.has_pending_prompt());
    multi.read_with(&cx, |multi, _| assert_eq!(multi.workspace(), &original));
}

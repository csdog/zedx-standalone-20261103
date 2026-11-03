use super::*;
use gpui::{TestAppContext, VisualTestContext};
use project::FakeFs;
use serde_json::json;

async fn setup(cx: &mut TestAppContext) -> (Arc<FakeFs>, Entity<ProjectPanel>, VisualTestContext) {
    crate::project_panel_tests::init_test(cx);
    cx.update(|cx| {
        workspace::register_project_item::<crate::project_panel_tests::TestProjectItemView>(cx);
    });
    let fs = FakeFs::new(cx.executor());
    fs.insert_tree("/project", json!({"main.rs":"fn main() {}"}))
        .await;
    let project = Project::test(fs.clone(), [Path::new("/project")], cx).await;
    let window =
        cx.add_window(|window, cx| workspace::MultiWorkspace::test_new(project, window, cx));
    let workspace = window
        .read_with(cx, |window, _| window.workspace().clone())
        .unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    let panel = workspace.update_in(&mut visual, ProjectPanel::new);
    workspace.update_in(&mut visual, |workspace, window, cx| {
        workspace.add_panel(panel.clone(), window, cx);
        workspace.open_panel::<ProjectPanel>(window, cx);
    });
    visual.run_until_parked();
    panel.update(&mut visual, |panel, cx| panel.refresh_zedx_groups(cx));
    (fs, panel, visual)
}

#[gpui::test]
async fn zedx_project_groups_follow_the_regular_tree_and_are_keyboard_reachable(
    cx: &mut TestAppContext,
) {
    let (_, panel, mut cx) = setup(cx).await;
    panel.read_with(&cx, |panel, _| {
        assert_eq!(
            panel
                .zedx_groups
                .rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            ["External Libraries", "Scratches and Consoles"]
        );
    });
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.select_last(&SelectLast, window, cx);
        assert!(panel.select_zedx_group_step(false, window, cx));
        assert_eq!(
            panel.zedx_groups.selected.as_deref(),
            Some("external-libraries")
        );
        assert!(panel.select_zedx_group_step(true, window, cx));
        assert!(panel.selection.is_some());
        assert!(panel.zedx_groups.selected.is_none());
    });
}

#[gpui::test]
async fn zedx_project_libraries_use_real_invisible_worktrees_without_relabeling_project_roots(
    cx: &mut TestAppContext,
) {
    let (fs, panel, mut cx) = setup(cx).await;
    fs.insert_tree("/dependencies", json!({"lib.rs":"pub fn dependency() {}"}))
        .await;
    let project = panel.read_with(&cx, |panel, _| panel.project.clone());
    let (external, _) = project
        .update(&mut cx, |project, cx| {
            project.find_or_create_worktree("/dependencies", false, cx)
        })
        .await
        .unwrap();
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.select_zedx_group(0, window, cx);
        panel.activate_zedx_group(Some(true), window, cx);
        panel.refresh_zedx_groups(cx);
    });
    panel.read_with(&cx, |panel, _| {
        assert!(
            panel
                .zedx_groups
                .rows
                .iter()
                .any(|row| row.label == "/dependencies")
        );
        assert!(
            !panel
                .zedx_groups
                .rows
                .iter()
                .any(|row| row.label == "/project")
        );
    });
    assert!(!external.read_with(&cx, |worktree, _| worktree.is_visible()));
}

#[gpui::test]
async fn zedx_project_scratch_creation_preserves_existing_files_and_stays_outside_project(
    cx: &mut TestAppContext,
) {
    let (fs, panel, mut cx) = setup(cx).await;
    let directory = paths::data_dir().join("scratches");
    fs.create_dir(&directory).await.unwrap();
    fs.atomic_write(
        directory.join("scratch-1.txt"),
        "preserve previous scratch".into(),
    )
    .await
    .unwrap();
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.load_zedx_scratches(true, window, cx)
    });
    cx.condition(&panel, |panel, _| !panel.zedx_groups.scratch_loading)
        .await;
    panel.read_with(&cx, |panel, _| {
        assert!(
            panel.zedx_groups.scratch_error.is_none(),
            "{:?}",
            panel.zedx_groups.scratch_error
        )
    });
    assert_eq!(
        fs.load(&directory.join("scratch-1.txt")).await.unwrap(),
        "preserve previous scratch"
    );
    assert_eq!(fs.load(&directory.join("scratch-2.txt")).await.unwrap(), "");
    assert!(
        fs.metadata(Path::new("/project/scratch-2.txt"))
            .await
            .unwrap()
            .is_none()
    );
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.load_zedx_scratches(false, window, cx)
    });
    cx.condition(&panel, |panel, _| !panel.zedx_groups.scratch_loading)
        .await;
    panel.read_with(&cx, |panel, cx| {
        let (worktree, _) = panel.zedx_groups.scratch.as_ref().unwrap();
        assert!(!worktree.read(cx).is_visible());
    });
}

#[gpui::test]
async fn zedx_project_groups_render_in_one_tree_and_expand_with_pointer(cx: &mut TestAppContext) {
    let (_, panel, mut cx) = setup(cx).await;
    cx.refresh().unwrap();
    let libraries = cx.debug_bounds("zedx-project-external-libraries").unwrap();
    let scratches = cx
        .debug_bounds("zedx-project-scratches-and-consoles")
        .unwrap();
    assert!(libraries.bottom() <= scratches.top());
    cx.simulate_click(scratches.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-project-consoles").is_some());
    let scratches = cx.debug_bounds("zedx-project-scratches").unwrap();
    cx.simulate_click(scratches.center(), gpui::Modifiers::none());
    cx.condition(&panel, |panel, _| !panel.zedx_groups.scratch_loading)
        .await;
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-project-new-scratch").is_none());
}

#[gpui::test]
async fn scratches_context_menu_new_file_creates_a_scratch_without_a_placeholder(
    cx: &mut TestAppContext,
) {
    let (fs, panel, mut cx) = setup(cx).await;
    panel.update_in(&mut cx, |panel, window, cx| {
        let index = panel
            .zedx_groups
            .rows
            .iter()
            .position(|row| matches!(row.target, Target::ScratchGroup))
            .unwrap();
        panel.select_zedx_group(index, window, cx);
        assert!(panel.open_zedx_scratch_menu(gpui::point(px(100.0), px(100.0)), window, cx));
    });
    cx.refresh().unwrap();
    let row = cx.debug_bounds("zedx-context-menu-row-0").unwrap();
    cx.simulate_click(row.center(), gpui::Modifiers::none());
    cx.condition(&panel, |panel, _| !panel.zedx_groups.scratch_loading)
        .await;
    assert_eq!(
        fs.load(&paths::data_dir().join("scratches/scratch-1.txt"))
            .await
            .unwrap(),
        ""
    );
    assert!(
        fs.metadata(Path::new("/project/scratch-1.txt"))
            .await
            .unwrap()
            .is_none()
    );
    panel.update(&mut cx, |panel, cx| {
        panel.refresh_zedx_groups(cx);
        assert!(panel.zedx_groups.expanded.contains("scratches"));
        assert!(
            !panel
                .zedx_groups
                .rows
                .iter()
                .any(|row| row.key == "new-scratch")
        );
        assert!(!panel.is_zedx_scratch_target(&Target::Libraries, cx));
    });
}

async fn setup_scratch_actions(
    cx: &mut TestAppContext,
) -> (Arc<FakeFs>, Entity<ProjectPanel>, VisualTestContext) {
    let (fs, panel, mut visual) = setup(cx).await;
    let directory = paths::data_dir().join("scratches");
    fs.create_dir(&directory).await.unwrap();
    fs.atomic_write(directory.join("notes.txt"), "scratch content".into())
        .await
        .unwrap();
    fs.create_dir(&directory.join("folder")).await.unwrap();
    panel.update_in(&mut visual, |panel, window, cx| {
        panel
            .zedx_groups
            .expanded
            .extend(["scratches-and-consoles".into(), "scratches".into()]);
        panel.load_zedx_scratches(false, window, cx);
    });
    visual
        .condition(&panel, |panel, _| !panel.zedx_groups.scratch_loading)
        .await;
    visual.run_until_parked();
    panel.update(&mut visual, |panel, cx| panel.refresh_zedx_groups(cx));
    (fs, panel, visual)
}

#[gpui::test]
async fn scratch_rename_edits_the_hidden_worktree_and_retains_tree_selection(
    cx: &mut TestAppContext,
) {
    let (fs, panel, mut cx) = setup_scratch_actions(cx).await;
    panel.update_in(&mut cx, |panel, window, cx| {
        let index = panel
            .zedx_groups
            .rows
            .iter()
            .position(|row| row.label == "notes.txt")
            .unwrap();
        panel.select_zedx_group(index, window, cx);
        assert!(panel.open_zedx_scratch_menu(point(px(100.), px(100.)), window, cx));
        panel.rename(&Rename, window, cx);
        assert!(panel.state.edit_state.is_some());
        assert!(panel.has_zedx_scratch_selection(cx));
        assert!(!panel.activate_zedx_group(None, window, cx));
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let rename = panel.update_in(&mut cx, |panel, window, cx| {
        assert!(panel.filename_editor.read(cx).is_focused(window));
        panel
            .filename_editor
            .update(cx, |editor, cx| editor.set_text("renamed.txt", window, cx));
        panel.confirm_edit(true, window, cx).unwrap()
    });
    rename.await.unwrap();
    cx.run_until_parked();
    panel.update(&mut cx, |panel, cx| {
        panel.refresh_zedx_groups(cx);
        let row = panel
            .zedx_groups
            .rows
            .iter()
            .find(|row| row.label == "renamed.txt")
            .unwrap();
        assert_eq!(panel.zedx_groups.selected.as_ref(), Some(&row.key));
    });
    let directory = paths::data_dir().join("scratches");
    assert_eq!(
        fs.load(&directory.join("renamed.txt")).await.unwrap(),
        "scratch content"
    );
    assert!(
        fs.metadata(&directory.join("notes.txt"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fs.metadata(Path::new("/project/renamed.txt"))
            .await
            .unwrap()
            .is_none()
    );
}

#[gpui::test]
async fn scratch_copy_and_cut_paste_target_the_selected_scratch_folder(cx: &mut TestAppContext) {
    let (fs, panel, mut cx) = setup_scratch_actions(cx).await;
    panel.update_in(&mut cx, |panel, window, cx| {
        let file = panel
            .zedx_groups
            .rows
            .iter()
            .position(|row| row.label == "notes.txt")
            .unwrap();
        panel.select_zedx_group(file, window, cx);
        panel.copy(&Copy, window, cx);
        assert!(panel.has_pasteable_content(cx));
        let folder = panel
            .zedx_groups
            .rows
            .iter()
            .position(|row| row.label == "folder")
            .unwrap();
        panel.select_zedx_group(folder, window, cx);
        panel.activate_zedx_group(Some(true), window, cx);
        panel.paste(&Paste, window, cx);
    });
    cx.run_until_parked();
    let directory = paths::data_dir().join("scratches");
    assert_eq!(
        fs.load(&directory.join("folder/notes.txt")).await.unwrap(),
        "scratch content"
    );
    assert_eq!(
        fs.load(&directory.join("notes.txt")).await.unwrap(),
        "scratch content"
    );
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.refresh_zedx_groups(cx);
        // Paste selects the newly copied entry. Cut it back into the scratch root,
        // where the existing name forces the native collision handling.
        assert!(panel.has_zedx_scratch_selection(cx));
        panel.cut(&Cut, window, cx);
        let root = panel
            .zedx_groups
            .rows
            .iter()
            .position(|row| row.key == "scratches")
            .unwrap();
        panel.select_zedx_group(root, window, cx);
        assert!(panel.open_zedx_scratch_menu(point(px(100.), px(100.)), window, cx));
        panel.paste(&Paste, window, cx);
    });
    cx.run_until_parked();
    assert!(
        fs.metadata(&directory.join("folder/notes.txt"))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fs.load(&directory.join("notes copy.txt")).await.unwrap(),
        "scratch content"
    );
    assert!(
        fs.metadata(Path::new("/project/notes.txt"))
            .await
            .unwrap()
            .is_none()
    );
}

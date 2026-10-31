use super::*;
use fs::FakeFs;
use gpui::{Modifiers, TestAppContext, VisualTestContext};
use settings::SettingsStore;
use std::time::Duration;
use util::path;

pub(crate) async fn setup(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    cx.update(|cx| {
        let settings = SettingsStore::test(cx);
        cx.set_global(settings);
        cx.set_global(db::AppDatabase::test_new());
        theme_settings::init(theme::LoadThemes::JustBase, cx);
        editor::init(cx);
        init(cx);
        bind_keys(cx);
    });
    let fs = FakeFs::new(cx.executor());
    fs.insert_tree(
        path!("/project"),
        serde_json::json!({
            "main.rs": "fn main() { println!(\"search fixture\"); }\n",
            "README.md": "search fixture\n",
            ".gitignore": "ignored_unique.rs\n",
            "ignored_unique.rs": "externalneedle\n"
        }),
    )
    .await;
    let project = Project::test(fs, [path!("/project").as_ref()], cx).await;
    let (multi_workspace, cx) =
        cx.add_window_view(|window, cx| workspace::MultiWorkspace::test_new(project, window, cx));
    let workspace =
        multi_workspace.read_with(cx, |multi_workspace, _| multi_workspace.workspace().clone());
    cx.run_until_parked();
    cx.refresh().unwrap();
    (workspace, cx)
}

fn double_shift(cx: &mut VisualTestContext) {
    for _ in 0..2 {
        cx.simulate_modifiers_change(Modifiers::shift());
        cx.executor().advance_clock(Duration::from_millis(50));
        cx.simulate_modifiers_change(Modifiers::none());
        cx.executor().advance_clock(Duration::from_millis(50));
    }
    cx.run_until_parked();
}

#[gpui::test]
async fn window_modifier_events_open_search_and_repeat_changes_scope(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    cx.run_until_parked();
    double_shift(cx);
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .expect("real window modifier events should open Search Everywhere");
    assert!(!modal.read_with(cx, |modal, cx| {
        modal.picker.read(cx).delegate.include_non_project
    }));
    double_shift(cx);
    let repeated = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .expect("repeated double Shift should preserve the modal");
    assert_eq!(modal.entity_id(), repeated.entity_id());
    assert!(repeated.read_with(cx, |modal, cx| {
        modal.picker.read(cx).delegate.include_non_project
    }));
}

#[gpui::test]
async fn files_shortcut_and_split_confirmation_open_a_second_pane(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    cx.simulate_keystrokes("cmd-p");
    cx.run_until_parked();
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .expect("Cmd+P should open ZedX Files");
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            assert!(picker.delegate.category == Category::Files);
            picker.set_query("main", window, cx);
        });
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        1
    );
    cx.simulate_keystrokes("shift-enter");
    cx.run_until_parked();
    assert_eq!(
        workspace.read_with(cx, |workspace, _| workspace.panes().len()),
        2
    );
}

#[gpui::test]
async fn all_includes_text_and_type_filter_removes_other_sources(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("fixture", window, cx));
    });
    cx.run_until_parked();
    let text_count = modal.read_with(cx, |modal, cx| {
        modal
            .picker
            .read(cx)
            .delegate
            .rows
            .iter()
            .filter(|row| matches!(row.target, Target::Text(..)))
            .count()
    });
    assert_eq!(
        text_count, 2,
        "All should include both real project text matches"
    );
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, _| {
            picker.delegate.file_filter = FileTypes::only(FileFilter::Rust)
        });
        modal.refresh(window, cx);
    });
    cx.run_until_parked();
    modal.read_with(cx, |modal, cx| {
        let rows = &modal.picker.read(cx).delegate.rows;
        assert_eq!(rows.len(), 1);
        assert!(rows[0].detail.contains("main.rs"));
    });
}

#[gpui::test]
async fn island_gap_drag_resizes_and_double_click_restores_default(cx: &mut TestAppContext) {
    use gpui::{MouseButton, MouseUpEvent, point, px};
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Left, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.toggle_dock(DockPosition::Left, window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let gap = cx
        .debug_bounds("zedx-left-resize")
        .expect("island gap must have an actual hit region");
    assert_eq!(gap.size.width, px(6.0));
    let start = gap.center();
    let end = point(start.x + px(50.0), start.y);
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        point(start.x + px(10.0), start.y),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    let width = workspace.read_with(cx, |workspace, cx| {
        workspace
            .panel_size_state::<TestPanel>(cx)
            .and_then(|state| state.size)
    });
    assert!(
        width.is_some_and(|width| width > px(340.0)),
        "drag must change stored size: {width:?}"
    );
    cx.refresh().unwrap();
    let gap = cx.debug_bounds("zedx-left-resize").unwrap();
    cx.simulate_event(MouseUpEvent {
        position: gap.center(),
        button: MouseButton::Left,
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    cx.run_until_parked();
    assert_eq!(
        workspace.read_with(cx, |workspace, cx| workspace
            .panel_size_state::<TestPanel>(cx)
            .and_then(|state| state.size)),
        Some(px(300.0))
    );
    cx.refresh().unwrap();
    let gap = cx.debug_bounds("zedx-left-resize").unwrap();
    cx.simulate_click(gap.center(), Modifiers::none());
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    assert_eq!(
        workspace.read_with(cx, |workspace, cx| workspace
            .panel_size_state::<TestPanel>(cx)
            .and_then(|state| state.size)),
        Some(px(308.0))
    );
}

#[gpui::test]
async fn scope_excludes_ignored_files_and_cancelled_queries_do_not_publish(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        open_category(workspace, Some(Category::Files), window, cx)
    });
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.set_query("ignored_unique", window, cx)
        })
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        0
    );
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, _| picker.delegate.include_non_project = true);
        modal.refresh(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        1
    );
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.delegate.category = Category::Text;
            picker.set_query("externalneedle", window, cx);
        });
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        1
    );
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, _| picker.delegate.include_non_project = false);
        modal.refresh(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        0
    );
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.delegate.category = Category::All;
            picker.update_matches("fixture".into(), window, cx);
            picker.update_matches("no_such_result_982746".into(), window, cx);
        });
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        0
    );
}

#[gpui::test]
async fn file_type_filter_applies_before_top_hundred_limit(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    let fs = workspace.read_with(cx, |workspace, cx| {
        workspace.project().read(cx).fs().clone()
    });
    let mut files = serde_json::Map::new();
    for index in 0..150 {
        files.insert(format!("same{index:03}.txt"), serde_json::json!(""));
    }
    files.insert("samezz.rs".into(), serde_json::json!(""));
    fs.as_fake()
        .insert_tree(path!("/project"), serde_json::Value::Object(files))
        .await;
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        open_category(workspace, Some(Category::Files), window, cx)
    });
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.delegate.file_filter = FileTypes::only(FileFilter::Rust);
            picker.set_query("same", window, cx);
        });
    });
    cx.run_until_parked();
    modal.read_with(cx, |modal, cx| {
        let rows = &modal.picker.read(cx).delegate.rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "samezz.rs");
    });
}

#[gpui::test]
async fn grouped_results_preserve_selection_and_multiselect_can_be_empty(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("fixture", window, cx))
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-search-group-text").is_some());
    let selected = modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.set_selected_index(1, None, false, window, cx);
            let selected = picker.delegate.rows[1].clone();
            let path = match &selected.target {
                Target::Text(buffer, _) => {
                    let file = buffer.read(cx).file().unwrap();
                    ProjectPath {
                        worktree_id: file.worktree_id(cx),
                        path: file.path().clone(),
                    }
                }
                _ => panic!("fixture query should yield text"),
            };
            let row = SearchDelegate::file_row(path, picker.delegate.project.read(cx), cx);
            SearchDelegate::publish(picker, picker.delegate.generation, vec![row], window, cx);
            selected
        })
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-search-row-2").is_some(),
        "streamed third row must be rendered before any later query completes"
    );
    modal.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert!(delegate.rows[delegate.selected].same_target(&selected));
        assert_eq!(delegate.section(0), "Files");
        assert_eq!(delegate.section(1), "Text");
    });
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, _| {
            picker.delegate.file_filter.toggle(FileFilter::All);
            picker.delegate.file_filter.toggle(FileFilter::Rust);
            picker.delegate.file_filter.toggle(FileFilter::Markdown);
        });
        modal.refresh(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        2
    );
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, _| {
            picker.delegate.file_filter.toggle(FileFilter::Rust);
            picker.delegate.file_filter.toggle(FileFilter::Markdown);
        });
        modal.refresh(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        0
    );
}

#[gpui::test]
async fn long_search_and_preview_fit_compact_viewports_and_restore_focus(cx: &mut TestAppContext) {
    use gpui::{px, size};
    let (workspace, cx) = setup(cx).await;
    let fs = workspace.read_with(cx, |workspace, cx| {
        workspace.project().read(cx).fs().clone()
    });
    fs.as_fake().insert_tree(path!("/project"), serde_json::json!({ "long.rs": (0..60).map(|index| format!("viewportfixture {index}\n")).collect::<String>() })).await;
    cx.run_until_parked();
    let previous = cx.update(|window, cx| {
        let focus = workspace.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        focus
    });
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.delegate.category = Category::Text;
            picker.set_query("viewportfixture", window, cx);
        })
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        60
    );
    cx.refresh().unwrap();
    let long_height = cx.debug_bounds("zedx-search-modal").unwrap().size.height;
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("println", window, cx))
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let short_height = cx.debug_bounds("zedx-search-modal").unwrap().size.height;
    assert!(
        short_height < long_height,
        "a one-match search should shrink: {short_height:?} vs {long_height:?}"
    );
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.set_query("viewportfixture", window, cx)
        })
    });
    cx.run_until_parked();
    for viewport in [
        size(px(1280.0), px(900.0)),
        size(px(700.0), px(600.0)),
        size(px(390.0), px(844.0)),
        size(px(390.0), px(500.0)),
    ] {
        cx.simulate_resize(viewport);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let bounds = cx.debug_bounds("zedx-search-modal").unwrap();
        assert!(
            bounds.left() >= px(0.0) && bounds.right() <= viewport.width,
            "horizontal overflow: {viewport:?} {bounds:?}"
        );
        assert!(
            bounds.bottom() <= viewport.height,
            "vertical overflow: {viewport:?} {bounds:?}"
        );
        let footer = cx.debug_bounds("zedx-search-footer").unwrap();
        assert!(
            footer.bottom() <= bounds.bottom(),
            "footer clipped: {viewport:?} popup {bounds:?} footer {footer:?}"
        );
        cx.dispatch_action(picker::TogglePreview);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let preview_bounds = cx.debug_bounds("zedx-search-modal").unwrap();
        assert!(
            preview_bounds.right() <= viewport.width && preview_bounds.bottom() <= viewport.height,
            "preview overflow: {viewport:?} {preview_bounds:?}"
        );
        let footer = cx.debug_bounds("zedx-search-footer").unwrap();
        assert!(
            footer.bottom() <= preview_bounds.bottom(),
            "preview footer clipped: {viewport:?} {preview_bounds:?} {footer:?}"
        );
        cx.dispatch_action(picker::TogglePreview);
        cx.run_until_parked();
    }
    cx.dispatch_action(menu::Cancel);
    cx.run_until_parked();
    assert!(workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<SearchWindow>(cx).is_none()
    }));
    assert!(
        cx.update(|window, _| previous.is_focused(window)),
        "Escape should restore the previous focus"
    );
}

#[gpui::test]
async fn compact_project_panel_overlays_editor_and_restores_desktop_size(cx: &mut TestAppContext) {
    use gpui::{px, size};
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Left, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.toggle_dock(DockPosition::Left, window, cx);
    });
    cx.simulate_resize(size(px(390.0), px(844.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    let editor = cx.debug_bounds("zedx-editor-island").unwrap();
    let overlay = cx.debug_bounds("zedx-project-overlay").unwrap();
    assert_eq!(
        editor.left(),
        px(44.0),
        "project overlay must not consume editor width"
    );
    assert_eq!(overlay.size.width, px(235.0));
    assert!(overlay.right() < px(390.0));
    assert!(cx.debug_bounds("zedx-left-resize").is_none());
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-project-overlay").is_none());
    assert_eq!(
        cx.debug_bounds("zedx-editor-island").unwrap().left(),
        px(350.0)
    );
    assert!(workspace.read_with(cx, |workspace, cx| {
        workspace
            .dock_at_position(DockPosition::Left)
            .read(cx)
            .is_open()
    }));
}

#[gpui::test]
async fn navigation_wraps_categories_reverse_and_actions_cannot_deploy_file_search(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("fixture", window, cx))
    });
    cx.run_until_parked();
    cx.dispatch_action(menu::SelectPrevious);
    cx.run_until_parked();
    modal.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert_eq!(delegate.selected, delegate.rows.len() - 1);
    });
    cx.dispatch_action(menu::SelectNext);
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.selected),
        0
    );
    cx.simulate_keystrokes("shift-tab");
    cx.run_until_parked();
    assert!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.category
            == Category::Text)
    );
    cx.simulate_keystrokes("tab");
    cx.run_until_parked();
    assert!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.category
            == Category::All)
    );
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("/主题", window, cx))
    });
    cx.run_until_parked();
    modal.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert!(!delegate.rows.is_empty());
        assert!(
            delegate
                .rows
                .iter()
                .all(|row| matches!(row.target, Target::Action(_)))
        );
        assert_eq!(delegate.section(0), "Actions");
    });
    let panes = workspace.read_with(cx, |workspace, _| workspace.panes().len());
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, _| picker.delegate.category = Category::Actions);
        modal.find_in_files(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        workspace.read_with(cx, |workspace, _| workspace.panes().len()),
        panes
    );
    assert!(workspace.read_with(cx, |workspace, cx| {
        workspace.active_modal::<SearchWindow>(cx).is_some()
    }));
    cx.dispatch_action(menu::Cancel);
    cx.run_until_parked();
    cx.simulate_keystrokes("ctrl-p");
    cx.run_until_parked();
    let files = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .expect("Ctrl+P should open ZedX Files");
    assert!(
        files.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.category
            == Category::Files)
    );
}

#[gpui::test]
async fn main_menu_actions_find_restore_layout_by_natural_chinese_phrase(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    cx.dispatch_action(workspace::zedx_actions::SearchActions);
    cx.run_until_parked();
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            assert!(picker.delegate.category == Category::Actions);
            picker.set_query("恢复布局", window, cx);
        });
    });
    cx.run_until_parked();
    modal.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert!(delegate.rows.iter().any(|row| matches!(&row.target, Target::Action(index) if delegate.actions[*index].1.name() == "zedx::RestoreDefaultLayout")));
    });
}

#[gpui::test]
async fn find_tool_button_transfers_query_to_the_left_panel(cx: &mut TestAppContext) {
    cx.update(crate::zedx_panels::init);
    let (workspace, cx) = setup(cx).await;
    workspace
        .update_in(cx, |_, window, cx| {
            cx.spawn_in(window, async move |workspace, cx| {
                crate::zedx_panels::initialize(workspace, cx.clone()).await
            })
        })
        .await
        .unwrap();
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("fixture", window, cx));
    });
    cx.run_until_parked();
    modal.update_in(cx, |modal, window, cx| modal.find_in_files(window, cx));
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        assert!(workspace.active_modal::<SearchWindow>(cx).is_none());
        let dock = workspace
            .dock_at_position(workspace::dock::DockPosition::Left)
            .read(cx);
        assert!(dock.is_open());
        let panel = dock.active_panel().unwrap();
        assert_eq!(panel.persistent_name(), "Find");
        assert_eq!(panel.icon_label(window, cx), Some("2".into()));
        assert_eq!(
            workspace.panes().len(),
            1,
            "Find results belong in the left tool window"
        );
    });
}

#[gpui::test]
async fn short_windows_keep_all_three_grouped_results_visible_with_preview(
    cx: &mut TestAppContext,
) {
    use gpui::size;
    let (workspace, cx) = setup(cx).await;
    let fs = workspace.read_with(cx, |workspace, cx| {
        workspace.project().read(cx).fs().clone()
    });
    fs.as_fake()
        .insert_tree(
            path!("/project"),
            serde_json::json!({"fixture.rs": "fn unrelated() {}\n"}),
        )
        .await;
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("fixture", window, cx))
    });
    cx.run_until_parked();
    assert_eq!(
        modal.read_with(cx, |modal, cx| modal.picker.read(cx).delegate.rows.len()),
        3
    );
    cx.dispatch_action(picker::SetPreviewBelow);
    for viewport in [
        size(px(852.0), px(485.0)),
        size(px(390.0), px(500.0)),
        size(px(700.0), px(600.0)),
    ] {
        cx.simulate_resize(viewport);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let footer = cx.debug_bounds("zedx-search-footer").unwrap();
        for (index, selector) in [
            "zedx-search-row-0",
            "zedx-search-row-1",
            "zedx-search-row-2",
        ]
        .into_iter()
        .enumerate()
        {
            let row = cx
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("row {index} not rendered at {viewport:?}"));
            assert!(
                row.bottom() <= footer.top(),
                "result clipped at {viewport:?}: {row:?}, footer {footer:?}"
            );
        }
    }
}

use super::*;
use gpui::{TestAppContext, VisualTestContext};
use workspace::{dock::PanelSizeState, zedx_actions::RestoreDefaultLayout};

pub(super) async fn setup(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    cx.update(init);
    let (workspace, cx) = crate::zedx_search::tests::setup(cx).await;
    let load = workspace.update_in(cx, |_, window, cx| {
        cx.spawn_in(window, async move |workspace, cx| {
            initialize(workspace, cx.clone()).await
        })
    });
    load.await.unwrap();
    cx.run_until_parked();
    (workspace, cx)
}

#[gpui::test]
async fn structure_problems_and_run_are_real_toggleable_panels(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    workspace.read_with(cx, |workspace, cx| {
        assert!(workspace.panel::<Structure>(cx).is_some());
        assert!(
            workspace
                .panel::<Problems>(cx)
                .unwrap()
                .read(cx)
                .rows
                .is_empty()
        );
        assert!(
            workspace
                .panel::<Run>(cx)
                .unwrap()
                .read(cx)
                .task_label(cx)
                .is_none()
        );
    });
    cx.dispatch_action(ToggleStructure);
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.dock_at_position(DockPosition::Left).read(cx);
        assert!(dock.is_open());
        assert_eq!(dock.active_panel().unwrap().persistent_name(), "Structure");
    });
    cx.dispatch_action(workspace::zedx_actions::FindInFiles::default());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-head-find").is_some(),
        "the Find window uses the shared tool window head too"
    );

    cx.dispatch_action(ToggleProblems);
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.dock_at_position(DockPosition::Bottom).read(cx);
        assert!(dock.is_open());
        assert_eq!(dock.active_panel().unwrap().persistent_name(), "Problems");
    });
}

#[gpui::test]
async fn restore_layout_resets_sizes_and_closes_right_dock(cx: &mut TestAppContext) {
    use workspace::dock::test::TestPanel;
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Right, 100, cx));
        workspace.add_panel(panel, window, cx);
        for dock in workspace.all_docks() {
            dock.update(cx, |dock, cx| dock.set_open(true, window, cx));
        }
        let size = PanelSizeState {
            size: Some(px(400.0)),
            flex: None,
        };
        workspace.set_panel_size_state::<Structure>(size, window, cx);
        workspace.set_panel_size_state::<Problems>(size, window, cx);
        workspace.set_panel_size_state::<TestPanel>(size, window, cx);
    });
    cx.dispatch_action(RestoreDefaultLayout);
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert_eq!(
            workspace.panel_size_state::<Structure>(cx).unwrap().size,
            Some(px(265.0))
        );
        assert_eq!(
            workspace.panel_size_state::<Problems>(cx).unwrap().size,
            Some(px(202.0))
        );
        assert_eq!(
            workspace.panel_size_state::<TestPanel>(cx).unwrap().size,
            Some(px(300.0))
        );
        assert!(
            workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .is_open()
        );
        assert!(
            workspace
                .dock_at_position(DockPosition::Bottom)
                .read(cx)
                .is_open()
        );
        assert!(
            !workspace
                .dock_at_position(DockPosition::Right)
                .read(cx)
                .is_open()
        );
        assert!(
            workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .panel_index_for_persistent_name("TestPanel", cx)
                .is_some()
        );
        assert!(
            workspace
                .dock_at_position(DockPosition::Right)
                .read(cx)
                .panel_index_for_persistent_name("TestPanel", cx)
                .is_none()
        );
    });
}

#[gpui::test]
async fn problems_refresh_from_project_diagnostics_and_navigate_to_source(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
    let store = project.read_with(cx, |project, _| project.lsp_store());
    let publish = |diagnostics, cx: &mut VisualTestContext| {
        store.update(cx, |store, cx| {
            store
                .update_diagnostics(
                    lsp::LanguageServerId(0),
                    lsp::PublishDiagnosticsParams {
                        uri: lsp::Uri::from_file_path(util::path!("/project/main.rs")).unwrap(),
                        diagnostics,
                        version: None,
                    },
                    None,
                    language::DiagnosticSourceKind::Pushed,
                    &[],
                    cx,
                )
                .unwrap();
        });
    };
    publish(
        vec![lsp::Diagnostic {
            range: lsp::Range::new(lsp::Position::new(0, 3), lsp::Position::new(0, 7)),
            severity: Some(lsp::DiagnosticSeverity::WARNING),
            message: "fixture warning".into(),
            ..Default::default()
        }],
        cx,
    );
    cx.dispatch_action(ToggleProblems);
    cx.run_until_parked();
    let panel = workspace.read_with(cx, |workspace, cx| workspace.panel::<Problems>(cx).unwrap());
    let row = panel.read_with(cx, |panel, cx| {
        assert_eq!(panel.badge(cx), Some("1".into()));
        assert_eq!(panel.rows.len(), 1);
        assert_eq!(panel.rows[0].message, "fixture warning");
        assert_eq!(panel.rows[0].location, "main.rs:1:4");
        panel.rows[0].clone()
    });
    panel.update_in(cx, |panel, window, cx| panel.open(row, window, cx));
    cx.run_until_parked();
    let editor = workspace.read_with(cx, |workspace, cx| {
        workspace.active_item_as::<Editor>(cx).unwrap()
    });
    editor.update(cx, |editor, cx| {
        let snapshot = editor
            .display_map
            .update(cx, |display, cx| display.snapshot(cx));
        let selection = editor.selections.newest::<language::Point>(&snapshot);
        assert_eq!(selection.start, language::Point::new(0, 3));
        assert_eq!(selection.end, language::Point::new(0, 7));
    });
    publish(Vec::new(), cx);
    cx.run_until_parked();
    panel.read_with(cx, |panel, cx| {
        assert!(panel.rows.is_empty());
        assert!(panel.badge(cx).is_none());
    });
}

#[gpui::test]
async fn status_tracks_real_unsaved_edits_and_file_indentation(cx: &mut TestAppContext) {
    use crate::zedx_status_bar::{Indentation, ProjectState};
    use settings::SettingsStore;
    use workspace::{SaveIntent, StatusItemView};
    let (workspace, cx) = setup(cx).await;
    let project_path = workspace.read_with(cx, |workspace, cx| ProjectPath {
        worktree_id: workspace
            .project()
            .read(cx)
            .worktrees(cx)
            .next()
            .unwrap()
            .read(cx)
            .id(),
        path: util::rel_path::rel_path("main.rs").into(),
    });
    workspace
        .update_in(cx, |workspace, window, cx| {
            workspace.open_path(project_path, None, true, window, cx)
        })
        .await
        .unwrap();
    let (state, indentation, editor) = workspace.update_in(cx, |workspace, window, cx| {
        let state = cx.new(|cx| ProjectState::new(workspace, cx));
        let indentation = cx.new(|_| Indentation::new(workspace));
        let editor = workspace.active_item_as::<Editor>(cx).unwrap();
        indentation.update(cx, |indentation, cx| {
            indentation.set_active_pane_item(Some(&editor), window, cx)
        });
        (state, indentation, editor)
    });
    assert_eq!(
        state.read_with(cx, |state, cx| state.labels(cx)),
        ("Local project".into(), false)
    );
    assert_eq!(
        indentation.read_with(cx, |indentation, cx| indentation.label(cx)),
        Some("4 spaces".into())
    );
    editor.update(cx, |editor, cx| {
        editor.buffer().update(cx, |buffer, cx| {
            buffer.edit(
                [(
                    editor::MultiBufferOffset(0)..editor::MultiBufferOffset(0),
                    "// unsaved\n",
                )],
                None,
                cx,
            )
        })
    });
    assert!(state.read_with(cx, |state, cx| state.labels(cx).1));
    workspace
        .update_in(cx, |workspace, window, cx| {
            workspace.save_active_item(SaveIntent::Save, window, cx)
        })
        .await
        .unwrap();
    assert!(!state.read_with(cx, |state, cx| state.labels(cx).1));
    cx.update(|_, cx| {
        cx.update_global::<SettingsStore, _>(|store, cx| {
            store.update_user_settings(cx, |settings| {
                settings.project.all_languages.defaults.tab_size = Some(2.try_into().unwrap());
                settings.project.all_languages.defaults.hard_tabs = Some(true);
            })
        })
    });
    assert_eq!(
        indentation.read_with(cx, |indentation, cx| indentation.label(cx)),
        Some("Tabs: 2".into())
    );
}

#[gpui::test]
async fn compact_docks_alternate_without_overlap_and_file_open_closes_project(
    cx: &mut TestAppContext,
) {
    use gpui::size;
    use workspace::dock::test::TestPanel;
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Right, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.focus_panel::<Structure>(window, cx);
    });
    cx.simulate_resize(size(px(390.0), px(844.0)));
    cx.run_until_parked();
    let state = |cx: &VisualTestContext| {
        workspace.read_with(cx, |workspace, cx| {
            (
                workspace
                    .dock_at_position(DockPosition::Left)
                    .read(cx)
                    .is_open(),
                workspace
                    .dock_at_position(DockPosition::Right)
                    .read(cx)
                    .is_open(),
            )
        })
    };
    assert_eq!(state(cx), (true, false));
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<TestPanel>(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(state(cx), (false, true));
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-project-overlay").is_none());
    assert!(cx.debug_bounds("zedx-right-overlay").is_some());
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<Structure>(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(state(cx), (true, false));
    let path = workspace.read_with(cx, |workspace, cx| ProjectPath {
        worktree_id: workspace
            .project()
            .read(cx)
            .worktrees(cx)
            .next()
            .unwrap()
            .read(cx)
            .id(),
        path: util::rel_path::rel_path("main.rs").into(),
    });
    workspace
        .update_in(cx, |workspace, window, cx| {
            workspace.open_path(path, None, false, window, cx)
        })
        .await
        .unwrap();
    cx.run_until_parked();
    assert_eq!(state(cx), (false, false));
    workspace.update_in(cx, |workspace, window, cx| {
        assert!(
            workspace
                .active_item_as::<Editor>(cx)
                .unwrap()
                .focus_handle(cx)
                .contains_focused(window, cx),
            "hidden dock must return keyboard focus to the opened file"
        );
    });
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<Structure>(window, cx);
    });
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<TestPanel>(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(state(cx), (true, true), "desktop keeps independent docks");
    assert_eq!(
        workspace.read_with(cx, |workspace, cx| workspace
            .panel_size_state::<Structure>(cx)
            .unwrap()
            .size),
        None
    );
}

#[gpui::test]
async fn status_notifies_when_an_inactive_editor_becomes_dirty(cx: &mut TestAppContext) {
    use crate::zedx_status_bar::ProjectState;
    use std::{cell::Cell, rc::Rc};
    use workspace::StatusItemView;
    let (workspace, cx) = setup(cx).await;
    let worktree_id = workspace.read_with(cx, |workspace, cx| {
        workspace
            .project()
            .read(cx)
            .worktrees(cx)
            .next()
            .unwrap()
            .read(cx)
            .id()
    });
    let mut editors = Vec::new();
    for name in ["main.rs", "README.md"] {
        let path = ProjectPath {
            worktree_id,
            path: util::rel_path::rel_path(name).into(),
        };
        let item = workspace
            .update_in(cx, |workspace, window, cx| {
                workspace.open_path(path, None, true, window, cx)
            })
            .await
            .unwrap();
        editors.push(item.downcast::<Editor>().unwrap());
    }
    let state = workspace.update_in(cx, |workspace, window, cx| {
        let state = cx.new(|cx| ProjectState::new(workspace, cx));
        state.update(cx, |state, cx| {
            state.set_active_pane_item(Some(&editors[1]), window, cx)
        });
        state
    });
    cx.run_until_parked();
    let notifications = Rc::new(Cell::new(0));
    let _observer = cx.update(|_, cx| {
        cx.observe(&state, {
            let notifications = notifications.clone();
            move |_, _| notifications.set(notifications.get() + 1)
        })
    });
    editors[0].update(cx, |editor, cx| {
        editor.buffer().update(cx, |buffer, cx| {
            buffer.edit(
                [(
                    editor::MultiBufferOffset(0)..editor::MultiBufferOffset(0),
                    "// inactive edit\n",
                )],
                None,
                cx,
            )
        })
    });
    cx.run_until_parked();
    assert!(state.read_with(cx, |state, cx| state.labels(cx).1));
    assert!(
        notifications.get() > 0,
        "save indicator must repaint for inactive-file changes"
    );
}

#[gpui::test]
async fn status_follows_repository_added_after_window_open_and_branch_changes(
    cx: &mut TestAppContext,
) {
    use crate::zedx_status_bar::ProjectState;
    let (workspace, cx) = setup(cx).await;
    let state = workspace.update(cx, |workspace, cx| {
        cx.new(|cx| ProjectState::new(workspace, cx))
    });
    let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
    let fs = project.read_with(cx, |project, _| project.fs().clone());
    fs.as_fake()
        .insert_tree(util::path!("/project"), serde_json::json!({".git": {}}))
        .await;
    fs.as_fake().set_branch_name(
        util::path!("/project/.git").as_ref(),
        Some("feat/idea-workbench"),
    );
    cx.run_until_parked();
    project
        .update(cx, |project, cx| project.git_scans_complete(cx))
        .await;
    cx.run_until_parked();
    assert_eq!(
        state.read_with(cx, |state, cx| state.labels(cx).0),
        "feat/idea-workbench"
    );
    fs.as_fake()
        .set_branch_name(util::path!("/project/.git").as_ref(), Some("main"));
    cx.run_until_parked();
    project
        .update(cx, |project, cx| project.git_scans_complete(cx))
        .await;
    cx.run_until_parked();
    assert_eq!(state.read_with(cx, |state, cx| state.labels(cx).0), "main");
}

#[gpui::test]
async fn notifications_paint_above_compact_docks_and_remain_clickable(cx: &mut TestAppContext) {
    use gpui::{Modifiers, size};
    use std::{cell::Cell, rc::Rc};
    use workspace::notifications::{
        NotificationId, simple_message_notification::MessageNotification,
    };
    let (workspace, cx) = setup(cx).await;
    cx.simulate_resize(size(px(390.0), px(500.0)));
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<Structure>(window, cx);
    });
    cx.run_until_parked();
    let clicks = Rc::new(Cell::new(0));
    let notification_clicks = clicks.clone();
    workspace.update(cx, |workspace, cx| {
        workspace.show_notification(NotificationId::unique::<Structure>(), cx, |cx| {
            cx.new(|cx| {
                MessageNotification::new_from_builder(cx, move |_, _| {
                    let clicks = notification_clicks.clone();
                    div()
                        .debug_selector(|| "zedx-notification-content".into())
                        .child(
                            Button::new("notification-action", "Notification above dock")
                                .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                        )
                        .into_any_element()
                })
            })
        });
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let notification = cx.debug_bounds("zedx-notification-content").unwrap();
    let overlay = cx.debug_bounds("zedx-project-overlay").unwrap();
    assert!(
        overlay.contains(&notification.center()),
        "fixture must overlap the dock"
    );
    cx.simulate_click(notification.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        clicks.get(),
        1,
        "notification must receive input above the deferred dock"
    );
    let layer = cx.debug_bounds("zedx-notifications").unwrap();
    assert!(layer.left() >= px(0.0) && layer.right() <= px(390.0));
}

#[gpui::test]
async fn project_header_actions_focus_the_real_tree_and_hide_its_dock(cx: &mut TestAppContext) {
    use gpui::Modifiers;
    use project_panel::ProjectPanel;
    cx.update(project_panel::init);
    let (workspace, cx) = setup(cx).await;
    let panel = workspace
        .update_in(cx, |_, window, cx| {
            cx.spawn_in(window, async move |workspace, cx| {
                ProjectPanel::load(workspace, cx.clone()).await
            })
        })
        .await
        .unwrap();
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.add_panel(panel.clone(), window, cx);
        workspace.focus_panel::<ProjectPanel>(window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let collapse = cx.debug_bounds("zedx-project-collapse").unwrap();
    cx.simulate_click(collapse.center(), Modifiers::none());
    cx.run_until_parked();
    workspace.update_in(cx, |_, window, cx| {
        assert!(panel.focus_handle(cx).contains_focused(window, cx))
    });
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-project-footer").is_some());
    let hide = cx.debug_bounds("zedx-project-hide").unwrap();
    cx.simulate_click(hide.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(!workspace.read_with(cx, |workspace, cx| {
        workspace
            .dock_at_position(DockPosition::Left)
            .read(cx)
            .is_open()
    }));
}

#[gpui::test]
async fn editor_tabs_close_and_split(cx: &mut TestAppContext) {
    use workspace::SplitDirection;
    let (workspace, cx) = setup(cx).await;
    let worktree_id = workspace.read_with(cx, |workspace, cx| {
        workspace
            .project()
            .read(cx)
            .worktrees(cx)
            .next()
            .unwrap()
            .read(cx)
            .id()
    });
    for name in ["main.rs", "README.md"] {
        let path = ProjectPath {
            worktree_id,
            path: util::rel_path::rel_path(name).into(),
        };
        workspace
            .update_in(cx, |workspace, window, cx| {
                workspace.open_path(path, None, false, window, cx)
            })
            .await
            .unwrap();
    }
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert_eq!(workspace.active_pane().read(cx).items().count(), 2);
    });
    workspace.update_in(cx, |workspace, window, cx| {
        workspace
            .active_pane()
            .update(cx, |pane, cx| {
                pane.close_active_item(&Default::default(), window, cx)
            })
            .detach();
    });
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert_eq!(workspace.active_pane().read(cx).items().count(), 1);
    });
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.split_pane(
            workspace.active_pane().clone(),
            SplitDirection::Right,
            window,
            cx,
        );
    });
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, _| {
        assert_eq!(workspace.panes().len(), 2);
    });
}

#[gpui::test]
async fn right_dock_gap_keyboard_resizes(cx: &mut TestAppContext) {
    use gpui::{Modifiers, px};
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Right, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.toggle_dock(DockPosition::Right, window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let gap = cx
        .debug_bounds("zedx-right-resize")
        .expect("right island gap must have an actual hit region");
    assert_eq!(gap.size.width, px(6.0));
    cx.simulate_click(gap.center(), Modifiers::none());
    cx.simulate_keystrokes("left");
    cx.run_until_parked();
    let width = workspace.read_with(cx, |workspace, cx| {
        workspace
            .panel_size_state::<TestPanel>(cx)
            .and_then(|state| state.size)
    });
    assert!(
        width.is_some_and(|width| width > px(300.0)),
        "keyboard resize must change stored size: {width:?}"
    );
}

#[gpui::test]
async fn bottom_hide_button_closes_bottom_dock(cx: &mut TestAppContext) {
    use gpui::Modifiers;
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.toggle_panel_focus::<crate::zedx_panels::Problems>(window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let hide = cx
        .debug_bounds("zedx-hide-bottom")
        .expect("bottom hide button must have a hit region");
    cx.simulate_click(hide.center(), Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(
            !workspace
                .dock_at_position(DockPosition::Bottom)
                .read(cx)
                .is_open()
        );
    });
}
#[gpui::test]
async fn project_panel_header_controls_render(cx: &mut TestAppContext) {
    use gpui::Modifiers;
    use project_panel::ProjectPanel;
    let (workspace, cx) = setup(cx).await;
    let load = workspace.update_in(cx, |_workspace, window, cx| {
        cx.spawn_in(window, async move |workspace, cx| {
            let panel = ProjectPanel::load(workspace.clone(), cx.clone())
                .await
                .unwrap();
            workspace.update_in(cx, |workspace, window, cx| {
                workspace.add_panel(panel, window, cx);
            })?;
            anyhow::Ok(())
        })
    });
    load.await.unwrap();
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        workspace
            .dock_at_position(DockPosition::Left)
            .update(cx, |dock, cx| {
                dock.set_open(true, window, cx);
                if let Some(index) = dock.panel_index_for_persistent_name("Project Panel", cx) {
                    dock.activate_panel(index, window, cx);
                }
            });
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-project-collapse").is_some());
    assert!(cx.debug_bounds("zedx-project-hide").is_some());
    let hide = cx.debug_bounds("zedx-project-hide").unwrap();
    cx.simulate_click(hide.center(), Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(
            !workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .is_open()
        );
    });
}

#[gpui::test]
async fn bottom_gap_keyboard_resizes(cx: &mut TestAppContext) {
    use gpui::{Modifiers, px};
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Bottom, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.toggle_dock(DockPosition::Bottom, window, cx);
        workspace
            .dock_at_position(DockPosition::Bottom)
            .update(cx, |dock, cx| {
                if let Some(index) = dock.panel_index_for_persistent_name("TestPanel", cx) {
                    dock.activate_panel(index, window, cx);
                }
            });
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let gap = cx
        .debug_bounds("zedx-bottom-resize")
        .expect("bottom island gap must have an actual hit region");
    assert_eq!(gap.size.height, px(6.0));
    cx.simulate_click(gap.center(), Modifiers::none());
    cx.simulate_keystrokes("up");
    cx.simulate_keystrokes("up");
    cx.run_until_parked();
    let height = workspace.read_with(cx, |workspace, cx| {
        workspace
            .panel_size_state::<TestPanel>(cx)
            .and_then(|state| state.size)
    });
    assert!(
        height.is_some_and(|height| height > px(300.0)),
        "keyboard resize must change stored size: {height:?}"
    );
}

#[gpui::test]
async fn compact_mode_prefers_focused_right_dock(cx: &mut TestAppContext) {
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Right, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.focus_panel::<Structure>(window, cx);
        workspace.focus_panel::<TestPanel>(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_resize(gpui::size(px(390.0), px(844.0)));
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(
            !workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .is_open()
        );
        assert!(
            workspace
                .dock_at_position(DockPosition::Right)
                .read(cx)
                .is_open()
        );
    });
}

#[gpui::test]
async fn compact_mode_prefers_focused_left_dock(cx: &mut TestAppContext) {
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Right, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.focus_panel::<TestPanel>(window, cx);
        workspace.focus_panel::<Structure>(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_resize(gpui::size(px(390.0), px(844.0)));
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(
            workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .is_open()
        );
        assert!(
            !workspace
                .dock_at_position(DockPosition::Right)
                .read(cx)
                .is_open()
        );
    });
}
#[gpui::test]
async fn right_gap_physical_drag_resizes(cx: &mut TestAppContext) {
    use gpui::{MouseButton, point, px};
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Right, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.toggle_dock(DockPosition::Right, window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let gap = cx
        .debug_bounds("zedx-right-resize")
        .expect("right island gap must have an actual hit region");
    let start = gap.center();
    let end = point(start.x - px(50.0), start.y);
    cx.simulate_mouse_move(start, None, gpui::Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
    cx.simulate_mouse_move(
        point(start.x - px(10.0), start.y),
        Some(MouseButton::Left),
        gpui::Modifiers::none(),
    );
    cx.simulate_mouse_move(end, Some(MouseButton::Left), gpui::Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();
    let width = workspace.read_with(cx, |workspace, cx| {
        workspace
            .panel_size_state::<TestPanel>(cx)
            .and_then(|state| state.size)
    });
    assert!(
        width.is_some_and(|width| width > px(340.0)),
        "right physical drag must change stored size: {width:?}"
    );
}

#[gpui::test]
async fn bottom_gap_physical_drag_resizes(cx: &mut TestAppContext) {
    use gpui::{MouseButton, point, px};
    use workspace::dock::{DockPosition, test::TestPanel};
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.set_random_database_id();
        let panel = cx.new(|cx| TestPanel::new(DockPosition::Bottom, 100, cx));
        workspace.add_panel(panel, window, cx);
        workspace.toggle_dock(DockPosition::Bottom, window, cx);
        workspace
            .dock_at_position(DockPosition::Bottom)
            .update(cx, |dock, cx| {
                if let Some(index) = dock.panel_index_for_persistent_name("TestPanel", cx) {
                    dock.activate_panel(index, window, cx);
                }
            });
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let gap = cx
        .debug_bounds("zedx-bottom-resize")
        .expect("bottom island gap must have an actual hit region");
    let start = gap.center();
    let end = point(start.x, start.y - px(50.0));
    cx.simulate_mouse_move(start, None, gpui::Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
    cx.simulate_mouse_move(
        point(start.x, start.y - px(10.0)),
        Some(MouseButton::Left),
        gpui::Modifiers::none(),
    );
    cx.simulate_mouse_move(end, Some(MouseButton::Left), gpui::Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();
    let height = workspace.read_with(cx, |workspace, cx| {
        workspace
            .panel_size_state::<TestPanel>(cx)
            .and_then(|state| state.size)
    });
    assert!(
        height.is_some_and(|height| height > px(340.0)),
        "bottom physical drag must change stored size: {height:?}"
    );
}

#[gpui::test]
async fn git_panel_loads_with_language_model_init(cx: &mut TestAppContext) {
    use git_ui::git_panel::GitPanel;
    let (workspace, cx) = setup(cx).await;
    cx.update(|_, cx| {
        language_model::init(cx);
    });
    let load = workspace.update_in(cx, |_workspace, window, cx| {
        cx.spawn_in(window, async move |workspace, cx| {
            let panel = GitPanel::load(workspace.clone(), cx.clone()).await.unwrap();
            workspace.update_in(cx, |workspace, window, cx| {
                workspace.add_panel(panel, window, cx);
            })?;
            anyhow::Ok(())
        })
    });
    load.await.unwrap();
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        workspace
            .dock_at_position(DockPosition::Left)
            .update(cx, |dock, cx| {
                dock.set_open(true, window, cx);
                if let Some(index) = dock.panel_index_for_persistent_name("GitPanel", cx) {
                    dock.activate_panel(index, window, cx);
                }
            });
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.dock_at_position(DockPosition::Left).read(cx);
        assert!(dock.is_open());
        assert_eq!(
            dock.active_panel().map(|panel| panel.persistent_name()),
            Some("GitPanel")
        );
    });
}
#[gpui::test]
async fn run_panel_toggles_bottom_dock(cx: &mut TestAppContext) {
    use workspace::zedx_actions::ToggleRun;
    let (workspace, cx) = setup(cx).await;
    cx.dispatch_action(ToggleRun);
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.dock_at_position(DockPosition::Bottom).read(cx);
        assert!(dock.is_open());
        assert_eq!(
            dock.active_panel().map(|panel| panel.persistent_name()),
            Some("Run")
        );
    });
}

#[gpui::test]
async fn workbench_rails_match_prototype_widths(cx: &mut TestAppContext) {
    use gpui::px;
    let (_, cx) = setup(cx).await;
    cx.refresh().unwrap();
    assert_eq!(
        cx.debug_bounds("zedx-left-rail").unwrap().size.width,
        px(44.0)
    );
    assert_eq!(
        cx.debug_bounds("zedx-right-rail").unwrap().size.width,
        px(42.0)
    );
}

#[gpui::test]
async fn more_tools_menu_reaches_panels_from_every_dock(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    cx.refresh().unwrap();
    let trigger = cx
        .debug_bounds("zedx-more-tools")
        .expect("the left rail should expose the More tool windows menu");
    cx.simulate_click(trigger.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("MENU_ITEM-Structure").is_some(),
        "left-dock panels belong to the menu"
    );
    assert!(
        cx.debug_bounds("MENU_ITEM-Problems").is_some(),
        "bottom-dock panels belong to the menu"
    );
    assert!(
        cx.debug_bounds("MENU_ITEM-Branches").is_some(),
        "the ZedX Branches window is listed like any other panel"
    );
    assert!(
        cx.debug_bounds("MENU_ITEM-Git history").is_some(),
        "the prototype's tool menu also lists the Git log entry, which is an action not a panel"
    );

    let structure = cx
        .debug_bounds("MENU_ITEM-Structure")
        .expect("Structure lives in the left dock and stays reachable");
    cx.simulate_click(structure.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.dock_at_position(DockPosition::Left).read(cx);
        assert!(dock.is_open());
        assert_eq!(dock.active_panel().unwrap().persistent_name(), "Structure");
    });
}

#[gpui::test]
async fn show_git_history_opens_the_git_panel_history_tab(cx: &mut TestAppContext) {
    use git_ui::git_panel::GitPanel;
    let (workspace, cx) = setup(cx).await;
    cx.update(|_, cx| {
        language_model::init(cx);
    });
    let load = workspace.update_in(cx, |_workspace, window, cx| {
        cx.spawn_in(window, async move |workspace, cx| {
            let panel = GitPanel::load(workspace.clone(), cx.clone()).await?;
            workspace.update_in(cx, |workspace, window, cx| {
                workspace.add_panel(panel, window, cx);
            })?;
            anyhow::Ok(())
        })
    });
    load.await.unwrap();
    cx.run_until_parked();

    // Reach the Git log through the tool window menu, the way the prototype lists it.
    cx.refresh().unwrap();
    let trigger = cx
        .debug_bounds("zedx-more-tools")
        .expect("more tools trigger");
    cx.simulate_click(trigger.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    let entry = cx
        .debug_bounds("MENU_ITEM-Git history")
        .expect("the menu lists Git history");
    cx.simulate_click(entry.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.dock_at_position(DockPosition::Left).read(cx);
        assert!(dock.is_open(), "the Git tool window should be revealed");
        assert_eq!(dock.active_panel().unwrap().persistent_name(), "GitPanel");
        let panel = workspace.panel::<GitPanel>(cx).unwrap();
        assert!(
            panel.read(cx).showing_history_tab(),
            "Show Git History must land on the History tab, not Changes"
        );
    });
}

#[gpui::test]
async fn branches_panel_lists_branches_of_a_repository_created_after_open(cx: &mut TestAppContext) {
    use picker::PickerDelegate;
    let (workspace, cx) = setup(cx).await;
    let panel = workspace
        .read_with(cx, |workspace, cx| workspace.panel::<Branches>(cx))
        .expect("the Branches tool window is registered");
    assert_eq!(
        panel.read_with(cx, |panel, cx| {
            panel
                .branches
                .read(cx)
                .picker
                .read(cx)
                .delegate
                .match_count()
        }),
        0,
        "without a repository the list stays empty"
    );

    let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
    let fs = project.read_with(cx, |project, _| project.fs().clone());
    fs.as_fake()
        .insert_tree(util::path!("/project"), serde_json::json!({".git": {}}))
        .await;
    fs.as_fake().set_branch_name(
        util::path!("/project/.git").as_ref(),
        Some("feat/idea-workbench"),
    );
    cx.run_until_parked();
    project
        .update(cx, |project, cx| project.git_scans_complete(cx))
        .await;
    cx.run_until_parked();

    assert!(
        panel.read_with(cx, |panel, cx| {
            panel
                .branches
                .read(cx)
                .picker
                .read(cx)
                .delegate
                .match_count()
        }) > 0,
        "a repository discovered after the window opened must rebuild the branch list"
    );

    cx.dispatch_action(ToggleBranches);
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.dock_at_position(DockPosition::Left).read(cx);
        assert!(dock.is_open());
        assert_eq!(dock.active_panel().unwrap().persistent_name(), "Branches");
    });
}

#[gpui::test]
async fn tool_window_heads_render_and_hide_their_panel(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;

    cx.dispatch_action(ToggleStructure);
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-head-structure").is_some(),
        "Structure uses the shared tool window head"
    );

    cx.dispatch_action(ToggleBranches);
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-head-branches").is_some(),
        "the Branches window renders the prototype's tool window head"
    );
    let hide = cx
        .debug_bounds("zedx-branches-hide")
        .expect("the head exposes a hide control");
    cx.simulate_click(hide.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(
            !workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .is_open(),
            "hiding from the head closes the left tool window"
        );
    });

    cx.dispatch_action(ToggleProblems);
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-head-problems").is_some(),
        "the Problems window renders the same head"
    );
    let hide = cx
        .debug_bounds("zedx-problems-hide")
        .expect("Problems exposes a hide control");
    cx.simulate_click(hide.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(
            !workspace
                .dock_at_position(DockPosition::Bottom)
                .read(cx)
                .is_open(),
            "hiding from the head closes the bottom tool window"
        );
    });
}

#[gpui::test]
async fn branches_panel_becomes_an_overlay_in_compact_mode(cx: &mut TestAppContext) {
    use gpui::{px, size};
    let (workspace, cx) = setup(cx).await;
    let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
    let fs = project.read_with(cx, |project, _| project.fs().clone());
    fs.as_fake()
        .insert_tree(util::path!("/project"), serde_json::json!({".git": {}}))
        .await;
    fs.as_fake().set_branch_name(
        util::path!("/project/.git").as_ref(),
        Some("feat/idea-workbench"),
    );
    cx.run_until_parked();
    project
        .update(cx, |project, cx| project.git_scans_complete(cx))
        .await;
    cx.run_until_parked();

    cx.dispatch_action(ToggleBranches);
    cx.run_until_parked();
    cx.simulate_resize(size(px(390.0), px(844.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    let overlay = cx
        .debug_bounds("zedx-project-overlay")
        .expect("the left dock becomes the compact overlay");
    assert_eq!(overlay.size.width, px(235.0));
    assert_eq!(
        cx.debug_bounds("zedx-editor-island").unwrap().left(),
        px(44.0),
        "the Branches overlay must not consume editor width"
    );

    cx.simulate_resize(size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-project-overlay").is_none(),
        "restoring the desktop size puts the panel back in the dock"
    );
}

#[gpui::test]
async fn more_tools_menu_follows_the_rail_order(cx: &mut TestAppContext) {
    use gpui::px;
    let (workspace, cx) = setup(cx).await;
    let _ = &workspace;
    cx.refresh().unwrap();

    let trigger = cx
        .debug_bounds("zedx-more-tools")
        .expect("more tools trigger");
    cx.simulate_click(trigger.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();

    // Problems is registered before Run, but the rail puts Run first; the menu must agree
    // with the rail instead of dock insertion order.
    let run = cx.debug_bounds("MENU_ITEM-Run").expect("Run is listed");
    let problems = cx
        .debug_bounds("MENU_ITEM-Problems")
        .expect("Problems is listed");
    assert!(
        run.top() < problems.top(),
        "menu order must follow rail priority (Run {}, Problems {})",
        run.top() / px(1.0),
        problems.top() / px(1.0)
    );
}

#[gpui::test]
async fn moving_a_tool_window_reorders_the_rail_and_the_menu(cx: &mut TestAppContext) {
    use gpui::px;
    use workspace::zedx_actions::MoveToolWindow;
    let (workspace, cx) = setup(cx).await;
    let _ = &workspace;
    cx.refresh().unwrap();

    let open_menu = |cx: &mut VisualTestContext| {
        let trigger = cx
            .debug_bounds("zedx-more-tools")
            .expect("more tools trigger");
        cx.simulate_click(trigger.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.refresh().unwrap();
    };

    open_menu(cx);
    let branches_before = cx
        .debug_bounds("MENU_ITEM-Branches")
        .expect("Branches listed");
    let structure_before = cx
        .debug_bounds("MENU_ITEM-Structure")
        .expect("Structure listed");
    assert!(
        structure_before.top() < branches_before.top(),
        "default order puts Structure before Branches"
    );
    // Toggle the popover shut before dispatching, otherwise the next click closes it again.
    let trigger = cx
        .debug_bounds("zedx-more-tools")
        .expect("more tools trigger");
    cx.simulate_click(trigger.center(), gpui::Modifiers::none());
    cx.run_until_parked();

    cx.dispatch_action(MoveToolWindow {
        name: "Branches".to_string(),
        delta: -3,
    });
    cx.run_until_parked();

    open_menu(cx);
    let branches_after = cx
        .debug_bounds("MENU_ITEM-Branches")
        .expect("Branches listed");
    let structure_after = cx
        .debug_bounds("MENU_ITEM-Structure")
        .expect("Structure listed");
    assert!(
        branches_after.top() < structure_after.top(),
        "after moving Branches up it must lead Structure in the menu ({} vs {})",
        branches_after.top() / px(1.0),
        structure_after.top() / px(1.0)
    );
}

#[gpui::test]
async fn restore_default_layout_forgets_the_custom_rail_order(cx: &mut TestAppContext) {
    use gpui::px;
    use workspace::zedx_actions::{MoveToolWindow, RestoreDefaultLayout};
    let (workspace, cx) = setup(cx).await;
    let _ = &workspace;
    cx.refresh().unwrap();

    let menu_order = |cx: &mut VisualTestContext| {
        let trigger = cx
            .debug_bounds("zedx-more-tools")
            .expect("more tools trigger");
        cx.simulate_click(trigger.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.refresh().unwrap();
        let bounds = (
            cx.debug_bounds("MENU_ITEM-Structure")
                .expect("Structure listed")
                .top(),
            cx.debug_bounds("MENU_ITEM-Branches")
                .expect("Branches listed")
                .top(),
        );
        let trigger = cx
            .debug_bounds("zedx-more-tools")
            .expect("more tools trigger");
        cx.simulate_click(trigger.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        bounds
    };

    let (structure_before, branches_before) = menu_order(cx);
    assert!(
        structure_before < branches_before,
        "default order leads with Structure"
    );

    cx.dispatch_action(MoveToolWindow {
        name: "Branches".to_string(),
        delta: -3,
    });
    cx.run_until_parked();
    let (structure_moved, branches_moved) = menu_order(cx);
    assert!(
        branches_moved < structure_moved,
        "the custom order puts Branches first ({} vs {})",
        branches_moved / px(1.0),
        structure_moved / px(1.0)
    );

    cx.dispatch_action(RestoreDefaultLayout);
    cx.run_until_parked();
    let (structure_after, branches_after) = menu_order(cx);
    assert!(
        structure_after < branches_after,
        "Restore Default Layout must forget the custom rail order ({} vs {})",
        structure_after / px(1.0),
        branches_after / px(1.0)
    );
}

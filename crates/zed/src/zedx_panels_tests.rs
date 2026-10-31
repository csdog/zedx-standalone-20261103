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

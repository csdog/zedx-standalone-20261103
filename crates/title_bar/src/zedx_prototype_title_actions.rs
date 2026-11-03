use super::*;
use ui::ButtonLike;
use workspace::zedx_actions::SearchEverywhere;

impl TitleBar {
    pub(super) fn render_zedx_actions(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let width = window.viewport_size().width;
        let controls = zedx_title_layout::control_size(window);
        let button = |id: &'static str, icon, label: &'static str, action: Box<dyn Action>| {
            let workspace = self.workspace.clone();
            div().debug_selector(move || id.into()).child(
                ButtonLike::new(id)
                    .width(controls.width)
                    .height(controls.height.into())
                    .aria_label(label)
                    .tab_index(0isize)
                    .child(ui::zedx::icon(icon, cx).size(px(20.0)))
                    .tooltip(Tooltip::text(label))
                    .on_click(move |_, window, cx| {
                        if let Some(workspace) = workspace.upgrade() {
                            window.focus(&workspace.read(cx).focus_handle(cx), cx);
                        }
                        window.dispatch_action(action.boxed_clone(), cx);
                    }),
            )
        };
        let workspace = self.workspace.clone();
        let recent_configuration = self.workspace.upgrade().and_then(|workspace| {
            workspace::zedx_run_history::recent(workspace.read(cx), cx)
                .into_iter()
                .next()
        });
        let last_task = self
            .project
            .read(cx)
            .task_store()
            .read(cx)
            .task_inventory()
            .and_then(|inventory| inventory.read(cx).last_scheduled_task(None))
            .map(|(_, task)| task.resolved_label);
        let task_label = last_task
            .clone()
            .or_else(|| {
                recent_configuration
                    .as_ref()
                    .map(|entry| entry.label.clone())
            })
            .unwrap_or_else(|| "Run configuration".to_string());
        let run_action: Box<dyn Action> = if last_task.is_none()
            && let Some(recent) = recent_configuration
        {
            workspace::zedx_run_history::RunRecentConfiguration { key: recent.key }.boxed_clone()
        } else {
            zed_actions::Rerun::default().boxed_clone()
        };
        h_flex()
            .debug_selector(|| "zedx-title-actions".into())
            .gap(px(if width <= px(1100.0) { 4.0 } else { 7.0 }))
            .pr(px(5.0))
            .when(width > px(760.0), |row| {
                row.child(
                    div()
                        .w(px(1.0))
                        .h(px(18.0))
                        .mx(px(5.0))
                        .bg(cx.theme().colors().border_variant),
                )
                .child(
                    div()
                        .debug_selector(|| "zedx-run-configuration".into())
                        .child(
                            ui::PopoverMenu::new("zedx-run-configuration-menu")
                                .trigger(
                                    ButtonLike::new("zedx-run-configuration")
                                        .height(px(30.0).into())
                                        .aria_label("Run configuration")
                                        .tab_index(0isize)
                                        .child(
                                            h_flex()
                                                .gap(px(7.0))
                                                .px(px(7.0))
                                                .max_w(px(220.0))
                                                .min_w_0()
                                                .child(
                                                    ui::zedx::icon(ui::zedx::Icon::Code, cx)
                                                        .size(px(16.0)),
                                                )
                                                .child(
                                                    Label::new(util::truncate_and_trailoff(
                                                        &task_label,
                                                        28,
                                                    ))
                                                    .size(ui::zedx::label_size())
                                                    .truncate(),
                                                )
                                                .child(Icon::new(IconName::ChevronDown).size(
                                                    IconSize::Custom(gpui::rems(13.0 / 16.0)),
                                                )),
                                        )
                                        .tooltip(Tooltip::text(format!(
                                            "Choose a project task · {task_label}"
                                        ))),
                                )
                                .menu(move |window, cx| {
                                    let workspace = workspace.clone();
                                    Some(ui::ContextMenu::build(window, cx, move |menu, _, cx| {
                                        let mut menu = menu;
                                        if let Some(workspace) = workspace.upgrade() {
                                            let recent = workspace::zedx_run_history::recent(workspace.read(cx), cx);
                                            if !recent.is_empty() {
                                                menu = menu.header("Recent Configurations");
                                                for entry in recent {
                                                    let workspace = workspace.downgrade();
                                                    menu = menu.entry(entry.label, None, move |window, cx| {
                                                        if let Some(workspace) = workspace.upgrade() {
                                                            window.focus(&workspace.read(cx).focus_handle(cx), cx);
                                                            window.dispatch_action(workspace::zedx_run_history::RunRecentConfiguration { key: entry.key.clone() }.boxed_clone(), cx);
                                                        }
                                                    });
                                                }
                                                menu = menu.separator();
                                            }
                                        }
                                        menu.entry("Choose Configuration…", None, {
                                            let workspace = workspace.clone();
                                            move |window, cx| {
                                                if let Some(workspace) = workspace.upgrade() {
                                                    window.focus(
                                                        &workspace.read(cx).focus_handle(cx),
                                                        cx,
                                                    );
                                                    window.dispatch_action(
                                                        zed_actions::Spawn::modal().boxed_clone(),
                                                        cx,
                                                    );
                                                }
                                            }
                                        })
                                        .separator()
                                        .entry("Edit Configurations…", None, {
                                            let workspace = workspace.clone();
                                            move |window, cx| {
                                                if let Some(workspace) = workspace.upgrade() {
                                                    window.focus(
                                                        &workspace.read(cx).focus_handle(cx),
                                                        cx,
                                                    );
                                                    window.dispatch_action(
                                                        zed_actions::OpenProjectTasks.boxed_clone(),
                                                        cx,
                                                    );
                                                }
                                            }
                                        })
                                        .entry(
                                            "Edit Debug Configurations…",
                                            None,
                                            move |window, cx| {
                                                if let Some(workspace) = workspace.upgrade() {
                                                    window.focus(
                                                        &workspace.read(cx).focus_handle(cx),
                                                        cx,
                                                    );
                                                    window.dispatch_action(
                                                        zed_actions::OpenProjectDebugTasks
                                                            .boxed_clone(),
                                                        cx,
                                                    );
                                                }
                                            },
                                        )
                                    }))
                                }),
                        ),
                )
            })
            .child(button(
                "zedx-run",
                ui::zedx::Icon::Play,
                "Run last task",
                run_action,
            ))
            .children(
                cx.build_action("debugger::Start", None)
                    .log_err()
                    .map(|action| {
                        button("zedx-debug", ui::zedx::Icon::Debug, "Debug project", action)
                    }),
            )
            .children(self.render_zedx_stop(window, cx))
            .child(
                div().debug_selector(|| "zedx-edit-run-configuration".into()).child(
                    ButtonLike::new("zedx-edit-run-configuration")
                        .width(controls.width)
                        .height(controls.height.into())
                        .aria_label("Edit Configurations")
                        .tab_index(0isize)
                        .tooltip(Tooltip::text("Edit Configurations…"))
                        .child(Icon::new(IconName::Pencil).size(IconSize::Custom(gpui::rems(20.0 / 16.0))))
                        .on_click({
                            let workspace = self.workspace.clone();
                            move |_, window, cx| {
                                if let Some(workspace) = workspace.upgrade() {
                                    window.focus(&workspace.read(cx).focus_handle(cx), cx);
                                    window.dispatch_action(zed_actions::OpenProjectTasks.boxed_clone(), cx);
                                }
                            }
                        }),
                ),
            )
            .when(width > px(760.0), |row| {
                row.child(
                    div()
                        .w(px(1.0))
                        .h(px(18.0))
                        .mx(px(5.0))
                        .bg(cx.theme().colors().border_variant),
                )
            })
            .child(button(
                "zedx-search",
                ui::zedx::Icon::Search,
                "Search Everywhere · Shift Shift",
                SearchEverywhere.boxed_clone(),
            ))
            .child(button(
                "zedx-settings",
                ui::zedx::Icon::Settings,
                "Open Settings",
                zed_actions::OpenSettings.boxed_clone(),
            ))
    }
}

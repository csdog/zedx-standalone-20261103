use super::*;
use ui::{ButtonLike, ButtonSize};

impl PanelButtons {
    pub(super) fn render_zedx(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let dock = self.dock.read(cx);
        let active_index = dock.active_panel_index;
        let is_open = dock.is_open;
        let dock_position = dock.position;
        let mut entries = dock.panel_entries.iter().enumerate().collect::<Vec<_>>();
        entries.sort_by_key(
            |(_, entry)| match (dock_position, entry.panel.persistent_name()) {
                (DockPosition::Left, "Project Panel")
                | (DockPosition::Right, "AgentPanel")
                | (DockPosition::Bottom, "Run") => 0,
                (DockPosition::Left, "Find")
                | (DockPosition::Right, "Outline Panel")
                | (DockPosition::Bottom, "TerminalPanel") => 1,
                (DockPosition::Left, "GitPanel") | (DockPosition::Bottom, "Problems") => 2,
                (DockPosition::Left, "Structure") => 3,
                (DockPosition::Bottom, "DebugPanel") => 3,
                _ => 4,
            },
        );
        let buttons = entries
            .into_iter()
            .filter(|(_, entry)| entry.panel.enabled(cx))
            .filter_map(|(index, entry)| {
                let icon = entry.panel.icon(window, cx)?;
                let title = entry.panel.icon_tooltip(window, cx)?;
                let panel = entry.panel.clone();
                let active = is_open && active_index == Some(index);
                let action = if active {
                    dock.toggle_action()
                } else {
                    panel.toggle_action(window, cx)
                };
                let click_action = action.boxed_clone();
                let panel_name = panel.persistent_name();
                let focus_handle = dock.focus_handle(cx);
                let badge = panel
                    .icon_label(window, cx)
                    .and_then(|label| label.parse::<usize>().ok());
                Some(
                    right_click_menu(panel.persistent_name())
                        .anchor(if dock_position == DockPosition::Right {
                            Anchor::TopRight
                        } else {
                            Anchor::TopLeft
                        })
                        .attach(if dock_position == DockPosition::Right {
                            Anchor::TopLeft
                        } else {
                            Anchor::TopRight
                        })
                        .menu(move |window, cx| {
                            ContextMenu::build(window, cx, |mut menu, _, cx| {
                                for position in [
                                    DockPosition::Left,
                                    DockPosition::Bottom,
                                    DockPosition::Right,
                                ] {
                                    if panel.position_is_valid(position, cx) {
                                        let panel = panel.clone();
                                        menu = menu.toggleable_entry(
                                            format!("Dock {}", position.label()),
                                            dock_position == position,
                                            IconPosition::Start,
                                            None,
                                            move |window, cx| {
                                                panel.set_position(position, window, cx)
                                            },
                                        );
                                    }
                                }
                                menu
                            })
                        })
                        .trigger(move |_, _, cx| {
                            div()
                                .relative()
                                .debug_selector(move || format!("zedx-tool-{panel_name}"))
                                .child(
                                    ButtonLike::new(("zedx-tool", index))
                                        .width(px(32.0))
                                        .height(px(32.0).into())
                                        .size(ButtonSize::Large)
                                        .child(match ui::zedx::tool_window_icon(panel_name) {
                                            Some(icon) => {
                                                ui::zedx::icon(icon, cx).into_any_element()
                                            }
                                            None => Icon::new(icon)
                                                .size(IconSize::Small)
                                                .into_any_element(),
                                        })
                                        .toggle_state(active)
                                        .aria_label(title)
                                        .tab_index(0isize)
                                        .tooltip({
                                            let action = action.boxed_clone();
                                            move |_, cx| Tooltip::for_action(title, &*action, cx)
                                        })
                                        .on_click({
                                            let action = click_action.boxed_clone();
                                            let focus_handle = focus_handle.clone();
                                            move |_, window, cx| {
                                                window.focus(&focus_handle, cx);
                                                window.dispatch_action(action.boxed_clone(), cx);
                                            }
                                        }),
                                )
                                .when_some(badge.filter(|_| !active), |this, count| {
                                    this.child(CountBadge::new(count))
                                })
                        }),
                )
            })
            .map(IntoElement::into_any_element)
            .collect::<Vec<_>>();
        v_flex().items_center().gap_1().children(buttons)
    }
}

impl Dock {
    pub(super) fn render_zedx_panel(
        &self,
        panel: &Arc<dyn PanelHandle>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let content = panel
            .to_any()
            .cached(StyleRefinement::default().v_flex().size_full());
        if !crate::zedx_workbench::enabled() || panel.persistent_name() != "Project Panel" {
            return content.into_any_element();
        }
        let focus = panel.activation_focus_handle(cx);
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h(px(28.0))
                    .flex_none()
                    .px_2()
                    .justify_between()
                    .child(Label::new("Project").size(LabelSize::Small))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                div()
                                    .debug_selector(|| "zedx-project-collapse".into())
                                    .child(
                                        ui::zedx::tool_button(
                                            "zedx-project-collapse",
                                            ui::zedx::Icon::Collapse,
                                            "Collapse all",
                                            cx,
                                        )
                                        .on_click(
                                            move |_, window, cx| {
                                                window.focus(&focus, cx);
                                                if let Some(action) = cx
                                                    .build_action(
                                                        "project_panel::CollapseAllEntries",
                                                        None,
                                                    )
                                                    .log_err()
                                                {
                                                    window.dispatch_action(action, cx);
                                                }
                                            },
                                        ),
                                    ),
                            )
                            .child(
                                div().debug_selector(|| "zedx-project-hide".into()).child(
                                    ui::zedx::tool_button(
                                        "zedx-project-hide",
                                        ui::zedx::Icon::Minus,
                                        "Hide Project",
                                        cx,
                                    )
                                    .on_click(cx.listener(
                                        |dock, _, window, cx| {
                                            dock.set_open(false, window, cx);
                                            let workspace = dock.workspace.clone();
                                            window.defer(cx, move |window, cx| {
                                                workspace
                                                    .update(cx, |workspace, cx| {
                                                        window
                                                            .focus(&workspace.focus_handle(cx), cx);
                                                        workspace.serialize_workspace(window, cx);
                                                    })
                                                    .log_err();
                                            });
                                        },
                                    )),
                                ),
                            ),
                    ),
            )
            .child(div().flex_1().min_h_0().child(content))
            .into_any_element()
    }

    pub(crate) fn reset_zedx_sizes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut states = Vec::new();
        for entry in &mut self.panel_entries {
            entry.panel.set_flexible_size(false, window, cx);
            let size = entry.panel.default_size(window, cx);
            states.push(resize_panel_entry(
                self.position,
                entry,
                Some(size),
                None,
                window,
                cx,
            ));
        }
        let workspace = self.workspace.clone();
        cx.defer(move |cx| {
            workspace
                .update(cx, |workspace, cx| {
                    for (key, state) in states {
                        workspace.persist_panel_size_state(key, state, cx);
                    }
                })
                .log_err();
        });
        cx.notify();
    }
}

impl Dock {
    pub(crate) fn zedx_tabs(&self, cx: &App) -> Vec<(usize, &'static str, bool)> {
        self.panel_entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.panel.enabled(cx))
            .filter_map(|(index, entry)| {
                let label = match entry.panel.persistent_name() {
                    "TerminalPanel" => "Terminal",
                    "Run" => "Run",
                    "Problems" => "Problems",
                    "DebugPanel" => "Debug",
                    _ => return None,
                };
                Some((
                    index,
                    label,
                    self.is_open && self.active_panel_index == Some(index),
                ))
            })
            .collect()
    }
}

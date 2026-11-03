use super::*;
use gpui::{InteractiveElement, StatefulInteractiveElement};
use std::{cell::Cell, rc::Rc};
use ui::{ButtonLike, ButtonSize};

#[path = "zedx_rail_drag.rs"]
mod rail_drag;
use rail_drag::RailDropTarget;

/// Rail order shared by the tool window bar and the `More tool windows` menu, so one panel cannot
/// sit in different positions in the two lists.
pub(crate) fn zedx_panel_priority(dock: DockPosition, name: &str) -> u8 {
    match (dock, name) {
        (DockPosition::Left, "Project Panel")
        | (DockPosition::Right, "AgentPanel")
        | (DockPosition::Bottom, "Git history") => 0,
        (DockPosition::Left, "GitPanel")
        | (DockPosition::Right, "Outline Panel")
        | (DockPosition::Bottom, "Run") => 1,
        (DockPosition::Left, "Structure") | (DockPosition::Bottom, "TerminalPanel") => 2,
        (DockPosition::Bottom, "Problems") => 3,
        (DockPosition::Bottom, "DebugPanel") => 4,
        _ => 4,
    }
}

/// Sort key shared by the rail, the `More tool windows` menu and reordering, so "the order the user
/// sees" is one definition rather than three.
pub(crate) fn zedx_rail_sort_key(
    order: Option<&crate::zedx_rail_order::RailOrder>,
    dock: DockPosition,
    name: &str,
) -> usize {
    order
        .and_then(|order| order.rank(dock, name))
        .unwrap_or(zedx_panel_priority(dock, name) as usize + 100)
}

fn zedx_panel_is_pinned(
    order: Option<&crate::zedx_rail_order::RailOrder>,
    dock: DockPosition,
    name: &str,
    cx: &App,
) -> bool {
    if matches!(name, "AgentPanel" | "Outline Panel" | "Database") {
        return false;
    }
    crate::zedx_tool_visibility::is_visible(name, zedx_panel_default_pinned(order, dock, name), cx)
}

pub(crate) fn zedx_panel_default_pinned(
    order: Option<&crate::zedx_rail_order::RailOrder>,
    dock: DockPosition,
    name: &str,
) -> bool {
    if matches!(name, "AgentPanel" | "Outline Panel") {
        return false;
    }
    matches!(
        name,
        "Project Panel"
            | "Git history"
            | "GitPanel"
            | "Structure"
            | "Run"
            | "TerminalPanel"
            | "Problems"
    ) || order.and_then(|order| order.rank(dock, name)).is_some()
}

pub(crate) fn zedx_toggle_action(
    panel: &dyn PanelHandle,
    window: &Window,
    cx: &App,
) -> Box<dyn gpui::Action> {
    if panel.persistent_name() == "AgentPanel" {
        Box::new(crate::zedx_actions::ToggleRightAssistant)
    } else if panel.persistent_name() == "Outline Panel" {
        Box::new(crate::zedx_actions::ToggleRightOutline)
    } else if panel.persistent_name() == "GitPanel" {
        Box::new(crate::zedx_actions::ToggleCommit)
    } else {
        panel.toggle_action(window, cx)
    }
}

impl PanelButtons {
    pub(super) fn render_zedx(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let dock_entity = self.dock.clone();
        let dock = self.dock.read(cx);
        let dock_position = dock.position;
        let mut entries = dock.panel_entries.iter().enumerate().collect::<Vec<_>>();
        let order = cx
            .try_global::<crate::zedx_rail_order::RailOrder>()
            .cloned();
        entries.sort_by_key(|(_, entry)| {
            zedx_rail_sort_key(order.as_ref(), dock_position, entry.panel.persistent_name())
        });
        let buttons = entries
            .into_iter()
            .filter(|(_, entry)| entry.panel.enabled(cx))
            .filter(|(_, entry)| {
                zedx_panel_is_pinned(
                    order.as_ref(),
                    dock_position,
                    entry.panel.persistent_name(),
                    cx,
                )
            })
            .filter_map(|(index, entry)| {
                let icon = entry.panel.icon(window, cx)?;
                let title = match entry.panel.persistent_name() {
                    "GitPanel" => "Commit",
                    "Project Panel" => "Project",
                    "Outline Panel" => "Outline",
                    "AgentPanel" => "AI Assistant",
                    _ => entry.panel.icon_tooltip(window, cx)?,
                };
                let panel = entry.panel.clone();
                let active = dock.zedx_panel_visible(entry.panel.as_ref());
                let action = if active {
                    dock.toggle_action()
                } else {
                    zedx_toggle_action(panel.as_ref(), window, cx)
                };
                let click_action = action.boxed_clone();
                let panel_name = panel.persistent_name();
                let focus_handle = dock.focus_handle(cx);
                let badge = panel
                    .icon_label(window, cx)
                    .and_then(|label| label.parse::<usize>().ok());
                let dock_for_drag = dock_entity.clone();
                let dock_for_click = dock_entity.clone();
                let panel_id = entry.panel.panel_id();
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
                            let target = Rc::new(RailDropTarget {
                                dock: dock_for_drag.clone(),
                                position: dock_position,
                                name: panel_name.to_string(),
                            });
                            let bounds = Rc::new(Cell::new(None));
                            div()
                                .relative()
                                .debug_selector(move || format!("zedx-tool-{panel_name}"))
                                .on_children_prepainted({
                                    let bounds = bounds.clone();
                                    move |children, _, _| bounds.set(children.first().copied())
                                })
                                .id(("zedx-tool-drag", index))
                                .on_drag(
                                    RailDrag {
                                        name: panel_name.to_string(),
                                    },
                                    |drag, _, _, cx| {
                                        cx.new(|_| RailDragPreview {
                                            name: drag.name.clone(),
                                        })
                                    },
                                )
                                .on_drag_move::<RailDrag>({
                                    let target = target.clone();
                                    move |event, window, cx| {
                                        let dragged = event.drag(cx).name.clone();
                                        target.reorder(
                                            &dragged,
                                            event.bounds,
                                            event.event.position,
                                            window,
                                            cx,
                                        )
                                    }
                                })
                                // A single move can start the drag immediately before mouse-up.
                                .on_drop::<RailDrag>(move |drag, window, cx| {
                                    if let Some(bounds) = bounds.get() {
                                        target.reorder(
                                            &drag.name,
                                            bounds,
                                            window.mouse_position(),
                                            window,
                                            cx,
                                        );
                                    }
                                })
                                .when(active, |this| {
                                    this.child(
                                        div()
                                            .debug_selector(move || {
                                                format!("zedx-selected-tool-{panel_name}")
                                            })
                                            .absolute()
                                            .inset_0()
                                            .rounded(px(6.0))
                                            .bg(ui::zedx::tool_window_selected_background()),
                                    )
                                })
                                .child(
                                    ButtonLike::new(("zedx-tool", index))
                                        .width(px(32.0))
                                        .height(px(34.0).into())
                                        .size(ButtonSize::None)
                                        .when(active, |button| {
                                            button.style(ui::ButtonStyle::Transparent)
                                        })
                                        .child(match ui::zedx::tool_window_icon(panel_name) {
                                            Some(icon) => {
                                                if active {
                                                    ui::zedx::tool_window_selected_icon(icon, cx)
                                                        .into_any_element()
                                                } else {
                                                    ui::zedx::icon(icon, cx).into_any_element()
                                                }
                                            }
                                            None => Icon::new(icon)
                                                .when(active, |icon| {
                                                    icon.color(Color::Custom(gpui::white()))
                                                })
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
                                            let dock = dock_for_click.clone();
                                            move |_, window, cx| {
                                                if dock.update(cx, |dock, cx| {
                                                    dock.toggle_zedx_split_pin(panel_id, window, cx)
                                                }) {
                                                    return;
                                                }
                                                window.focus(&focus_handle, cx);
                                                window.dispatch_action(action.boxed_clone(), cx);
                                            }
                                        }),
                                )
                                .when_some(badge.filter(|count| *count > 0), |this, count| {
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
        if crate::zedx_workbench::enabled()
            && matches!(panel.persistent_name(), "AgentPanel" | "Outline Panel")
        {
            let assistant = panel.persistent_name() == "AgentPanel";
            return v_flex()
                .size_full()
                .min_h_0()
                .child(
                    ui::zedx::tool_window_head(
                        if assistant {
                            "assistant"
                        } else {
                            "right-outline"
                        },
                        if assistant {
                            ui::zedx::Icon::Spark
                        } else {
                            ui::zedx::Icon::Structure
                        },
                        if assistant { "Assistant" } else { "Structure" },
                        cx,
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .debug_selector(move || {
                                if assistant {
                                    "zedx-assistant-hide".into()
                                } else {
                                    "zedx-right-outline-hide".into()
                                }
                            })
                            .child(
                                ui::zedx::tool_button(
                                    if assistant {
                                        "zedx-assistant-hide"
                                    } else {
                                        "zedx-right-outline-hide"
                                    },
                                    ui::zedx::Icon::Minus,
                                    "Hide tool window",
                                    cx,
                                )
                                .on_click(cx.listener(
                                    |dock, _, window, cx| {
                                        dock.set_open(false, window, cx);
                                        let workspace = dock.workspace.clone();
                                        window.defer(cx, move |window, cx| {
                                            workspace
                                                .update(cx, |workspace, cx| {
                                                    window.focus(
                                                        &workspace
                                                            .active_pane()
                                                            .read(cx)
                                                            .focus_handle(cx),
                                                        cx,
                                                    );
                                                    workspace.serialize_workspace(window, cx);
                                                })
                                                .log_err();
                                        });
                                    },
                                )),
                            ),
                    ),
                )
                .child(div().flex_1().min_h_0().child(content))
                .into_any_element();
        }
        if !crate::zedx_workbench::enabled() || panel.persistent_name() != "Project Panel" {
            return content.into_any_element();
        }
        let focus = panel.activation_focus_handle(cx);
        let project_id = panel.panel_id();
        let worktree_count = self
            .workspace
            .upgrade()
            .map(|workspace| {
                workspace
                    .read(cx)
                    .project()
                    .read(cx)
                    .visible_worktrees(cx)
                    .count()
            })
            .unwrap_or(0);
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h(px(28.0))
                    .flex_none()
                    .px_2()
                    .justify_between()
                    .child(
                        Label::new("Project")
                            .size(ui::zedx::label_size())
                            .weight(gpui::FontWeight::MEDIUM),
                    )
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
                                        move |dock, _, window, cx| {
                                            if dock.close_zedx_split_panel(project_id, window, cx) {
                                                return;
                                            }
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
            .child(
                h_flex()
                    .debug_selector(|| "zedx-project-footer".into())
                    .h(px(24.0))
                    .flex_none()
                    .px_2()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Label::new("Workspace")
                            .size(ui::zedx::caption_size())
                            .color(Color::Muted),
                    )
                    .child(div().flex_1())
                    .child(
                        Label::new(format!("{worktree_count} worktrees"))
                            .size(ui::zedx::caption_size())
                            .color(Color::Muted),
                    ),
            )
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

impl PanelButtons {
    /// Reorders `name` inside this dock by `delta` rail positions, starting from the order the user
    /// currently sees so a first move is not a jump into an empty list.
    /// This dock's panels in the order the rail shows them.
    fn visible_names(&self, cx: &App) -> (DockPosition, Vec<String>) {
        let dock = self.dock.read(cx);
        let order = cx.try_global::<crate::zedx_rail_order::RailOrder>();
        let mut visible = dock
            .panel_handles()
            .iter()
            .map(|panel| panel.persistent_name().to_string())
            .collect::<Vec<_>>();
        visible.sort_by_key(|name| zedx_rail_sort_key(order, dock.position, name));
        (dock.position, visible)
    }

    pub(crate) fn move_panel(&mut self, name: &str, delta: isize, cx: &mut Context<Self>) {
        let (position, visible) = self.visible_names(cx);
        if visible.is_empty() {
            return;
        }
        let mut order = cx
            .try_global::<crate::zedx_rail_order::RailOrder>()
            .cloned()
            .unwrap_or_default();
        order.nudge(position, &visible, name, delta);
        cx.set_global(order);
        cx.notify();
    }
}

/// Payload carried while a rail icon is dragged.
pub(crate) use crate::dock::zedx_split::PanelDrag as RailDrag;

struct RailDragPreview {
    name: String,
}

impl Render for RailDragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_1()
            .py_0p5()
            .rounded(px(6.0))
            .bg(cx.theme().colors().element_background)
            .child(match ui::zedx::tool_window_icon(&self.name) {
                Some(icon) => ui::zedx::icon(icon, cx).into_any_element(),
                None => ui::zedx::icon(ui::zedx::Icon::More, cx).into_any_element(),
            })
    }
}

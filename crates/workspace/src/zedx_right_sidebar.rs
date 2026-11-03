use super::*;
use crate::{
    dock::PanelHandle,
    notifications::NotificationId,
    zedx_actions::{ToggleRightAssistant, ToggleRightOutline},
};
use std::{cell::Cell, rc::Rc, sync::Arc};

const RIGHT_CONTROLS: [&str; 3] = ["AgentPanel", "Outline Panel", "Database"];

pub(super) struct RightPanelActivation {
    panel_id: EntityId,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct RightRailDrag {
    name: String,
    label: &'static str,
}
struct RightRailDragPreview(&'static str);
impl Render for RightRailDragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(cx.theme().colors().surface_background)
            .child(Label::new(self.0).size(ui::zedx::label_size()))
    }
}

impl Workspace {
    pub(super) fn observe_zedx_right_sidebar(&self, cx: &mut Context<Self>) {
        if !super::zedx_workbench::enabled() {
            return;
        }
        for dock in self.all_docks() {
            cx.observe(dock, |_, _, cx| cx.notify()).detach();
        }
        cx.observe_global::<crate::zedx_tool_visibility::ToolVisibility>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::zedx_rail_order::RailOrder>(|_, cx| cx.notify())
            .detach();
    }

    fn zedx_right_panel(&self, name: &str, cx: &App) -> Option<Arc<dyn PanelHandle>> {
        self.all_docks()
            .iter()
            .flat_map(|dock| dock.read(cx).panel_handles())
            .find(|panel| panel.persistent_name() == name && panel.enabled(cx))
    }

    pub(super) fn toggle_zedx_right_panel(
        &mut self,
        name: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.zedx_right_panel_activation = None;
        let Some(panel) = self.zedx_right_panel(name, cx) else {
            self.show_toast(Toast::new(NotificationId::unique::<RightPanelActivation>(),
                if name == "AgentPanel" { "AI Assistant is unavailable. Check AI settings or wait for it to finish loading." }
                else if name == "Database" { "Database tool window is still loading." }
                else { "File outline is still loading." }).autohide(), cx);
            return;
        };
        let right = self.right_dock.read(cx);
        if right.is_open()
            && right
                .active_panel()
                .is_some_and(|active| active.panel_id() == panel.panel_id())
        {
            self.right_dock
                .update(cx, |dock, cx| dock.set_open(false, window, cx));
            window.focus(&self.active_pane.focus_handle(cx), cx);
            self.serialize_workspace(window, cx);
            cx.notify();
            return;
        }
        let subscriptions = self
            .all_docks()
            .into_iter()
            .map(|dock| {
                cx.observe_in(dock, window, |_, _, window, cx| {
                    cx.defer_in(window, |workspace, window, cx| {
                        workspace.finish_zedx_right_panel_activation(window, cx)
                    });
                })
            })
            .collect();
        self.zedx_right_panel_activation = Some(RightPanelActivation {
            panel_id: panel.panel_id(),
            _subscriptions: subscriptions,
        });
        if panel.position(window, cx) != DockPosition::Right {
            if !panel.position_is_valid(DockPosition::Right, cx) {
                self.zedx_right_panel_activation = None;
                return;
            }
            panel.set_position(DockPosition::Right, window, cx);
        }
        cx.defer_in(window, |workspace, window, cx| {
            workspace.finish_zedx_right_panel_activation(window, cx)
        });
    }

    fn finish_zedx_right_panel_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pending) = &self.zedx_right_panel_activation else {
            return;
        };
        let Some(panel) = self
            .right_dock
            .read(cx)
            .panel_for_id(pending.panel_id)
            .cloned()
        else {
            return;
        };
        self.zedx_right_panel_activation = None;
        self.right_dock.update(cx, |dock, cx| {
            if let Some(index) = dock.panel_index_for_persistent_name(panel.persistent_name(), cx) {
                dock.activate_panel(index, window, cx);
                dock.set_open(true, window, cx);
            }
        });
        window.focus(&panel.activation_focus_handle(cx), cx);
        self.serialize_workspace(window, cx);
        cx.notify();
    }

    pub(super) fn zedx_right_control_order(&self, cx: &App) -> Vec<String> {
        let mut names = RIGHT_CONTROLS
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let order = cx.try_global::<crate::zedx_rail_order::RailOrder>();
        names.sort_by_key(|name| {
            order
                .and_then(|order| order.rank(DockPosition::Right, name))
                .unwrap_or(
                    100 + RIGHT_CONTROLS
                        .iter()
                        .position(|default| *default == name.as_str())
                        .unwrap_or(3),
                )
        });
        names
    }

    pub(super) fn move_zedx_right_control(
        &mut self,
        name: &str,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        if !RIGHT_CONTROLS.contains(&name) {
            return false;
        }
        let names = self.zedx_right_control_order(cx);
        let mut order = cx
            .try_global::<crate::zedx_rail_order::RailOrder>()
            .cloned()
            .unwrap_or_default();
        let index = names.iter().position(|entry| entry == name).unwrap_or(0);
        let target = (index as isize + delta).clamp(0, names.len() as isize - 1) as usize;
        order.move_subset_to(DockPosition::Right, &names, name, target);
        cx.set_global(order);
        cx.notify();
        true
    }

    fn reorder_zedx_right_control(
        &mut self,
        dragged: &str,
        target_name: &str,
        bounds: gpui::Bounds<Pixels>,
        position: gpui::Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if dragged == target_name || !bounds.contains(&position) {
            return;
        }
        let names = self.zedx_right_control_order(cx);
        let Some(from) = names.iter().position(|name| name == dragged) else {
            return;
        };
        let Some(over) = names.iter().position(|name| name == target_name) else {
            return;
        };
        let target = over + usize::from(position.y > bounds.center().y);
        let target = target - usize::from(from < target);
        if target == from {
            return;
        }
        let mut order = cx
            .try_global::<crate::zedx_rail_order::RailOrder>()
            .cloned()
            .unwrap_or_default();
        order.move_subset_to(DockPosition::Right, &names, dragged, target);
        cx.set_global(order);
        cx.notify();
    }

    pub(super) fn render_zedx_right_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let controls = self
            .zedx_right_control_order(cx)
            .into_iter()
            .filter(|name| crate::zedx_tool_visibility::is_visible(name, true, cx))
            .map(|name| {
                let (id, glyph, label, action): (_, _, _, Box<dyn gpui::Action>) =
                    match name.as_str() {
                        "AgentPanel" => (
                            "zedx-tool-AgentPanel",
                            ui::zedx::Icon::Spark,
                            "AI Assistant",
                            Box::new(ToggleRightAssistant),
                        ),
                        "Outline Panel" => (
                            "zedx-tool-Outline Panel",
                            ui::zedx::Icon::Structure,
                            "File outline",
                            Box::new(ToggleRightOutline),
                        ),
                        _ => (
                            "zedx-database",
                            ui::zedx::Icon::Database,
                            "Database",
                            Box::new(super::zedx_actions::ShowDatabase),
                        ),
                    };
                let panel = self.zedx_right_panel(&name, cx);
                let available = panel.is_some();
                let right = self.right_dock.read(cx);
                let selected = right.is_open()
                    && panel.as_ref().is_some_and(|panel| {
                        right
                            .active_panel()
                            .is_some_and(|active| active.panel_id() == panel.panel_id())
                    });
                let selection_name = name.clone();
                let drop_name = name.clone();
                let bounds = Rc::new(Cell::new(None));
                div()
                    .relative()
                    .w(px(32.0))
                    .h(px(34.0))
                    .flex_none()
                    .on_children_prepainted({
                        let bounds = bounds.clone();
                        move |children, _, _| bounds.set(children.first().copied())
                    })
                    .id(id)
                    .debug_selector(move || id.into())
                    .on_drag(RightRailDrag { name, label }, |drag, _, _, cx| {
                        cx.new(|_| RightRailDragPreview(drag.label))
                    })
                    .on_drag_move::<RightRailDrag>(cx.listener({
                        let drop_name = drop_name.clone();
                        move |workspace, event: &gpui::DragMoveEvent<RightRailDrag>, _, cx| {
                            let dragged = event.drag(cx).name.clone();
                            workspace.reorder_zedx_right_control(
                                &dragged,
                                &drop_name,
                                event.bounds,
                                event.event.position,
                                cx,
                            );
                        }
                    }))
                    .on_drop::<RightRailDrag>(cx.listener(
                        move |workspace, drag: &RightRailDrag, window, cx| {
                            if let Some(bounds) = bounds.get() {
                                workspace.reorder_zedx_right_control(
                                    &drag.name,
                                    &drop_name,
                                    bounds,
                                    window.mouse_position(),
                                    cx,
                                );
                            }
                        },
                    ))
                    .when(selected, |control| {
                        control.child(
                            div()
                                .debug_selector(move || {
                                    format!("zedx-selected-right-{selection_name}")
                                })
                                .absolute()
                                .inset_0()
                                .rounded(px(6.0))
                                .bg(ui::zedx::tool_window_selected_background()),
                        )
                    })
                    .child(
                        ui::ButtonLike::new(id)
                            .width(px(32.0))
                            .height(px(34.0).into())
                            .size(ui::ButtonSize::None)
                            .when(selected, |button| {
                                button.style(ui::ButtonStyle::Transparent)
                            })
                            .aria_label(label)
                            .tab_index(0isize)
                            .disabled(!available)
                            .toggle_state(selected)
                            .child(if selected {
                                ui::zedx::tool_window_selected_icon(glyph, cx).into_any_element()
                            } else {
                                ui::zedx::icon(glyph, cx).into_any_element()
                            })
                            .tooltip(ui::Tooltip::text(if available {
                                label
                            } else if id == "zedx-tool-AgentPanel" {
                                "AI Assistant is disabled or still loading"
                            } else if id == "zedx-database" {
                                "Database tool window is still loading"
                            } else {
                                "File outline is still loading"
                            }))
                            .on_click(cx.listener(move |workspace, _, window, cx| {
                                window.focus(&workspace.focus_handle(cx), cx);
                                window.dispatch_action(action.boxed_clone(), cx);
                            })),
                    )
            })
            .collect::<Vec<_>>();
        v_flex().gap(px(4.0)).items_center().children(controls)
    }
}

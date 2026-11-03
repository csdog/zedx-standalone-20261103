#[path = "zedx_more_tools.rs"]
mod more_tools;

use super::*;
use gpui::{MouseUpEvent, deferred};
use std::sync::OnceLock;

pub(crate) const LEFT_TOOL_BAR_WIDTH: f32 = 44.0;
pub(crate) const RIGHT_TOOL_BAR_WIDTH: f32 = 42.0;

pub(crate) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("ZEDX_UPSTREAM_UI").is_none())
}

pub(crate) fn frame_inset() -> Pixels {
    px(if enabled() { 6.0 } else { 0.0 })
}

pub(crate) fn left_rail_offset() -> Pixels {
    px(if enabled() { LEFT_TOOL_BAR_WIDTH } else { 0.0 })
}

pub(crate) fn right_rail_offset() -> Pixels {
    px(if enabled() { RIGHT_TOOL_BAR_WIDTH } else { 0.0 })
}

impl Workspace {
    pub(super) fn render_zedx_notifications(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let notifications = (!self.notifications.center.open || !enabled())
            .then(|| self.render_notifications(window, cx))
            .flatten()
            .map(|notifications| {
                if enabled() {
                    // Compact docks are deferred too; messages must paint above them.
                    deferred(
                        notifications
                            .w(px(448.0)
                                .min((window.viewport_size().width - px(24.0)).max(px(1.0))))
                            .debug_selector(|| "zedx-notifications".into()),
                    )
                    .with_priority(1)
                    .into_any_element()
                } else {
                    notifications.into_any_element()
                }
            });
        let center = self.render_zedx_notification_center(window, cx);
        if notifications.is_none() && center.is_none() {
            return None;
        }
        Some(
            div()
                .absolute()
                .inset_0()
                .children(notifications)
                .children(center)
                .into_any_element(),
        )
    }

    fn zedx_action_button(
        &self,
        id: &'static str,
        icon: ui::zedx::Icon,
        label: &'static str,
        action: Box<dyn Action>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focus = self.focus_handle(cx);
        div()
            .relative()
            .debug_selector(move || id.into())
            .when(
                id == "zedx-notifications-button" && self.notifications.center.open,
                |button| {
                    button
                        .rounded(px(6.0))
                        .bg(cx.theme().colors().element_selected)
                },
            )
            .when(
                id == "zedx-notifications-button" && self.notifications.center.unread_count() > 0,
                |button| {
                    button.child(
                        div()
                            .absolute()
                            .right(px(3.0))
                            .top(px(3.0))
                            .size(px(5.0))
                            .rounded_full()
                            .bg(cx.theme().colors().text_accent),
                    )
                },
            )
            .child(
                ui::ButtonLike::new(id)
                    .width(px(32.0))
                    .height(px(32.0).into())
                    .aria_label(label)
                    .tab_index(0isize)
                    .child(ui::zedx::icon(icon, cx))
                    .tooltip(ui::Tooltip::text(label))
                    .on_click(move |_, window, cx| {
                        window.focus(&focus, cx);
                        window.dispatch_action(action.boxed_clone(), cx);
                    }),
            )
    }

    fn zedx_resize_gap(
        &self,
        position: DockPosition,
        dock: &Entity<Dock>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let dock = dock.clone();
        let keyboard_dock = dock.clone();
        div()
            .id(match position {
                DockPosition::Left => "zedx-left-resize",
                DockPosition::Right => "zedx-right-resize",
                DockPosition::Bottom => "zedx-bottom-resize",
            })
            .debug_selector(|| match position {
                DockPosition::Left => "zedx-left-resize".into(),
                DockPosition::Right => "zedx-right-resize".into(),
                DockPosition::Bottom => "zedx-bottom-resize".into(),
            })
            .flex_none()
            .when(position == DockPosition::Bottom, |gap| {
                gap.h(px(6.0)).w_full().cursor_row_resize()
            })
            .when(position != DockPosition::Bottom, |gap| {
                gap.w(px(6.0)).h_full().cursor_col_resize()
            })
            .bg(cx.theme().colors().title_bar_background)
            .focusable()
            .tab_index(0isize)
            .aria_label(match position {
                DockPosition::Left => "Resize left tool window",
                DockPosition::Right => "Resize right tool window",
                DockPosition::Bottom => "Resize bottom tool window",
            })
            .on_key_down(
                cx.listener(move |workspace, event: &gpui::KeyDownEvent, window, cx| {
                    let modifiers = event.keystroke.modifiers;
                    if modifiers.platform
                        || modifiers.control
                        || modifiers.alt
                        || modifiers.function
                    {
                        return;
                    }
                    if event.keystroke.key == "home" {
                        keyboard_dock.update(cx, |dock, cx| dock.reset_panel_sizes(window, cx));
                    } else {
                        let direction = match (position, event.keystroke.key.as_str()) {
                            (DockPosition::Left, "right")
                            | (DockPosition::Right, "left")
                            | (DockPosition::Bottom, "up") => 1.0,
                            (DockPosition::Left, "left")
                            | (DockPosition::Right, "right")
                            | (DockPosition::Bottom, "down") => -1.0,
                            _ => return,
                        };
                        let Some(size) = workspace.dock_size(&keyboard_dock.read(cx), window, cx)
                        else {
                            return;
                        };
                        let size = size + px(if modifiers.shift { 24.0 } else { 8.0 }) * direction;
                        let size = if position == DockPosition::Left {
                            size.clamp(px(180.0), px(420.0))
                        } else {
                            size
                        };
                        workspace.resize_dock(position, size, window, cx);
                    }
                    workspace.serialize_workspace(window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_drag(DraggedDock(position), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |workspace, event: &MouseUpEvent, window, cx| {
                    if event.click_count == 2 {
                        dock.update(cx, |dock, cx| dock.reset_panel_sizes(window, cx));
                        workspace.serialize_workspace(window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .occlude()
    }

    /// Every enabled tool window in rail order, tagged with the dock it lives in, so one menu can
    /// reach panels the rails do not show as buttons.
    fn zedx_tool_window_entries(
        &self,
        cx: &App,
    ) -> Vec<(
        DockPosition,
        String,
        bool,
        std::sync::Arc<dyn crate::dock::PanelHandle>,
    )> {
        let mut entries = Vec::new();
        for (position, dock) in [
            (DockPosition::Left, &self.left_dock),
            (DockPosition::Bottom, &self.bottom_dock),
            (DockPosition::Right, &self.right_dock),
        ] {
            let dock = dock.read(cx);
            let active = dock.active_panel().map(|panel| panel.persistent_name());
            let open = dock.is_open();
            let mut panels = dock.panel_handles();
            let order = cx
                .try_global::<crate::zedx_rail_order::RailOrder>()
                .cloned();
            panels.sort_by_key(|panel| {
                crate::dock::zedx_tool_window_bar::zedx_rail_sort_key(
                    order.as_ref(),
                    position,
                    panel.persistent_name(),
                )
            });
            for panel in panels {
                if !panel.enabled(cx) {
                    continue;
                }
                let name = panel.persistent_name();
                let selected = open && active == Some(name);
                entries.push((position, name.to_string(), selected, panel));
            }
        }
        entries
    }

    pub(crate) fn move_zedx_tool_window(
        &mut self,
        name: &str,
        delta: isize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.move_zedx_right_control(name, delta, cx) {
            return;
        }
        for (index, position) in [
            DockPosition::Left,
            DockPosition::Bottom,
            DockPosition::Right,
        ]
        .into_iter()
        .enumerate()
        {
            let holds_panel = self
                .dock_at_position(position)
                .read(cx)
                .panel_handles()
                .iter()
                .any(|panel| panel.persistent_name() == name);
            if !holds_panel {
                continue;
            }
            let buttons = self.zedx_panel_buttons[index].clone();
            buttons.update(cx, |buttons, cx| buttons.move_panel(name, delta, cx));
            cx.notify();
            return;
        }
    }

    fn zedx_more_tools(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_zedx_more_tools(cx)
    }

    pub(crate) fn render_zedx_workbench(
        &self,
        pane_render_context: &PaneRenderContext,
        centered_paddings: (Option<Div>, Option<Div>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let compact = window.viewport_size().width < px(560.0);
        let overlay_width = px(235.0).min(
            (window.viewport_size().width - px(LEFT_TOOL_BAR_WIDTH + RIGHT_TOOL_BAR_WIDTH))
                .max(px(1.0)),
        );
        let colors = cx.theme().colors();
        let frame_background = colors.title_bar_background;
        let editor_background = colors.editor_background;
        let left_open = self.left_dock.read(cx).is_open();
        let right_open = self.right_dock.read(cx).is_open();
        let bottom_open = self.bottom_dock.read(cx).is_open();
        let left = self
            .render_dock(DockPosition::Left, &self.left_dock, window, cx)
            .map(|dock| {
                dock.relative()
                    .rounded(px(10.0))
                    .when(left_open, |dock| {
                        dock.min_w(px(180.0))
                            .max_w(px(420.0))
                            .child(ui::zedx::material_corners(px(10.0), frame_background))
                    })
                    .when(compact && left_open, |dock| {
                        dock.absolute()
                            .left(px(LEFT_TOOL_BAR_WIDTH))
                            .top(frame_inset())
                            .bottom(frame_inset())
                            .w(overlay_width)
                            .min_w(px(0.0))
                            .max_w(overlay_width)
                            .shadow_lg()
                            .occlude()
                            .debug_selector(|| "zedx-project-overlay".into())
                    })
                    .into_any_element()
            });
        let right = self
            .render_dock(DockPosition::Right, &self.right_dock, window, cx)
            .map(|dock| {
                dock.relative()
                    .rounded(px(10.0))
                    .when(right_open, |dock| {
                        dock.child(ui::zedx::material_corners(px(10.0), frame_background))
                    })
                    .when(compact && right_open, |dock| {
                        dock.debug_selector(|| "zedx-right-overlay".into())
                            .absolute()
                            .right(px(RIGHT_TOOL_BAR_WIDTH))
                            .top(frame_inset())
                            .bottom(frame_inset())
                            .w(overlay_width)
                            .min_w(px(0.0))
                            .max_w(overlay_width)
                            .shadow_lg()
                            .occlude()
                    })
                    .into_any_element()
            });
        let bottom_height = self
            .dock_size(&self.bottom_dock.read(cx), window, cx)
            .unwrap_or(px(202.0));
        let bottom = self
            .render_dock(DockPosition::Bottom, &self.bottom_dock, window, cx)
            .map(|dock| {
                if !bottom_open {
                    return dock.into_any_element();
                }
                v_flex()
                    .relative()
                    .w_full()
                    .h(bottom_height)
                    .min_h_0()
                    .flex_none()
                    .overflow_hidden()
                    .rounded(px(10.0))
                    .children(self.render_zedx_bottom_tool_window_head(cx))
                    .child(dock.h_full().flex_1().min_h_0())
                    .child(ui::zedx::island_corners(px(10.0), frame_background))
                    .into_any_element()
            });
        let center = h_flex()
            .debug_selector(|| "zedx-editor-island".into())
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .rounded(px(10.0))
            .bg(editor_background)
            .when_some(centered_paddings.0, |this, padding| this.child(padding))
            .child(self.render_center(pane_render_context, window, cx))
            .when_some(centered_paddings.1, |this, padding| this.child(padding))
            .child(ui::zedx::island_corners(px(10.0), frame_background));

        h_flex()
            .relative()
            .items_stretch()
            .size_full()
            .min_h_0()
            // Keep the native backdrop exposed beneath side docks. Painting the frame
            // beneath them compounds two translucent fills and hides the blur.
            .py(frame_inset())
            .child(
                div()
                    .absolute()
                    .top_0()
                    .w_full()
                    .h(frame_inset())
                    .bg(frame_background),
            )
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .w_full()
                    .h(frame_inset())
                    .bg(frame_background),
            )
            // A coalesced first move can start a drag immediately before release,
            // without another drag-move event reaching the workspace capture handler.
            .on_drop::<DraggedDock>(cx.listener(|workspace, dragged: &DraggedDock, window, cx| {
                if workspace.zoomed.is_some() {
                    return;
                }
                let pointer = window.mouse_position();
                let size = match dragged.0 {
                    DockPosition::Left => pointer.x - workspace.bounds.left() - left_rail_offset(),
                    DockPosition::Right => {
                        workspace.bounds.right() - pointer.x - right_rail_offset()
                    }
                    DockPosition::Bottom => workspace.bounds.bottom() - pointer.y - frame_inset(),
                };
                workspace.resize_dock(dragged.0, size, window, cx);
                workspace.serialize_workspace(window, cx);
                cx.stop_propagation();
            }))
            .child(
                v_flex()
                    .debug_selector(|| "zedx-left-rail".into())
                    .bg(frame_background)
                    .w(px(LEFT_TOOL_BAR_WIDTH))
                    .flex_none()
                    .h_full()
                    .items_center()
                    .py_1()
                    .child(self.zedx_panel_buttons[0].clone())
                    .child(self.zedx_more_tools(cx))
                    .child(div().flex_1())
                    .child(self.zedx_panel_buttons[1].clone()),
            )
            .children(left.map(|dock| {
                if compact {
                    deferred(dock).into_any_element()
                } else {
                    dock
                }
            }))
            .when(left_open && !compact, |row| {
                row.child(self.zedx_resize_gap(DockPosition::Left, &self.left_dock, cx))
            })
            .child(
                v_flex()
                    .bg(frame_background)
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .child(center)
                    .when(bottom_open, |column| {
                        column.child(self.zedx_resize_gap(
                            DockPosition::Bottom,
                            &self.bottom_dock,
                            cx,
                        ))
                    })
                    .children(bottom),
            )
            .when(right_open && !compact, |row| {
                row.child(self.zedx_resize_gap(DockPosition::Right, &self.right_dock, cx))
            })
            .children(right.map(|dock| {
                if compact {
                    deferred(dock).into_any_element()
                } else {
                    dock
                }
            }))
            .child(
                v_flex()
                    .debug_selector(|| "zedx-right-rail".into())
                    .gap(px(4.0))
                    .bg(frame_background)
                    .w(px(RIGHT_TOOL_BAR_WIDTH))
                    .flex_none()
                    .h_full()
                    .items_center()
                    .py_1()
                    .when(
                        crate::zedx_tool_visibility::is_visible("Notifications", true, cx),
                        |rail| {
                            rail.child(self.zedx_action_button(
                                "zedx-notifications-button",
                                ui::zedx::Icon::Bell,
                                "Notifications",
                                super::zedx_actions::ShowNotifications.boxed_clone(),
                                cx,
                            ))
                        },
                    )
                    .child(self.render_zedx_right_sidebar(cx))
                    .child(self.zedx_panel_buttons[2].clone())
                    .child(div().flex_1()),
            )
    }
}

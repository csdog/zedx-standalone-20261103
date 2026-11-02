use crate::{Toast, Workspace, dock::DockPosition, notifications::NotificationId};
use gpui::{Context, Focusable, Window};

fn zedx_default_dock_position(name: &str) -> DockPosition {
    match name {
        "AgentPanel" | "Outline Panel" => DockPosition::Right,
        "TerminalPanel" | "DebugPanel" | "Run" | "Problems" => DockPosition::Bottom,
        _ => DockPosition::Left,
    }
}

impl Workspace {
    pub(super) fn restore_zedx_layout(
        &mut self,
        _: &super::zedx_actions::RestoreDefaultLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Move panels first; the dock observer re-docks them when their settings land.
        for position in [
            DockPosition::Left,
            DockPosition::Bottom,
            DockPosition::Right,
        ] {
            let dock = self.dock_at_position(position).clone();
            let panels = dock.read(cx).panel_handles();
            for panel in panels {
                let target = zedx_default_dock_position(panel.persistent_name());
                if target != position && panel.position_is_valid(target, cx) {
                    panel.set_position(target, window, cx);
                }
            }
        }
        for pane in &self.panes {
            pane.update(cx, |pane, cx| pane.set_zoomed(false, cx));
        }
        self.zoomed = None;
        self.zoomed_position = None;
        cx.defer_in(window, |workspace, window, cx| {
            for (position, preferred, open) in [
                (DockPosition::Left, "Project Panel", true),
                (DockPosition::Bottom, "TerminalPanel", true),
                (DockPosition::Right, "AgentPanel", false),
            ] {
                workspace.dock_at_position(position).update(cx, |dock, cx| {
                    dock.zoom_out(window, cx);
                    if let Some(index) = dock.panel_index_for_persistent_name(preferred, cx) {
                        dock.activate_panel(index, window, cx);
                    }
                    dock.reset_zedx_sizes(window, cx);
                    dock.set_open(open && dock.panels_len() > 0, window, cx);
                });
            }
            window.focus(&workspace.active_pane.focus_handle(cx), cx);
            workspace.serialize_workspace(window, cx);
            cx.notify();
        });
        let mut order = cx
            .try_global::<crate::zedx_rail_order::RailOrder>()
            .cloned()
            .unwrap_or_default();
        order.reset();
        cx.set_global(order);

        self.show_toast(
            Toast::new(
                NotificationId::unique::<super::zedx_actions::RestoreDefaultLayout>(),
                "Default workbench layout restored",
            )
            .autohide(),
            cx,
        );
        cx.notify();
    }
}

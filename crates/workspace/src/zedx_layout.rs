use crate::{Toast, Workspace, dock::DockPosition, notifications::NotificationId};
use gpui::{Context, Focusable, Window};

impl Workspace {
    pub(super) fn restore_zedx_layout(
        &mut self,
        _: &super::zedx_actions::RestoreDefaultLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for pane in &self.panes {
            pane.update(cx, |pane, cx| pane.set_zoomed(false, cx));
        }
        self.zoomed = None;
        self.zoomed_position = None;
        for (position, preferred, open) in [
            (DockPosition::Left, "Project Panel", true),
            (DockPosition::Bottom, "TerminalPanel", true),
            (DockPosition::Right, "AgentPanel", false),
        ] {
            self.dock_at_position(position).update(cx, |dock, cx| {
                dock.zoom_out(window, cx);
                if let Some(index) = dock.panel_index_for_persistent_name(preferred, cx) {
                    dock.activate_panel(index, window, cx);
                }
                dock.reset_zedx_sizes(window, cx);
                dock.set_open(open && dock.panels_len() > 0, window, cx);
            });
        }
        window.focus(&self.active_pane.focus_handle(cx), cx);
        self.serialize_workspace(window, cx);
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

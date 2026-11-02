use crate::{Toast, Workspace, dock::DockPosition, notifications::NotificationId};
use gpui::{Context, EntityId, Focusable, Subscription, Window};

pub(super) struct LayoutRestore {
    targets: Vec<(EntityId, DockPosition)>,
    _subscriptions: Vec<Subscription>,
}

fn zedx_default_dock_position(name: &str) -> DockPosition {
    match name {
        "AgentPanel" | "Outline Panel" => DockPosition::Right,
        "TerminalPanel" | "DebugPanel" | "Run" | "Problems" | "Git history" => DockPosition::Bottom,
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
        self.left_dock
            .update(cx, |dock, cx| dock.reset_zedx_split(window, cx));
        // Settings-backed panels move asynchronously. Reset sizes only after the
        // dock observers have transferred every panel to its destination.
        let subscriptions = self
            .all_docks()
            .into_iter()
            .map(|dock| {
                cx.observe_in(dock, window, |_, _, window, cx| {
                    cx.defer_in(window, |workspace, window, cx| {
                        workspace.finish_zedx_layout_restore(window, cx);
                    });
                })
            })
            .collect();
        self.zedx_layout_restore = Some(LayoutRestore {
            targets: Vec::new(),
            _subscriptions: subscriptions,
        });
        for position in [
            DockPosition::Left,
            DockPosition::Bottom,
            DockPosition::Right,
        ] {
            let panels = self.dock_at_position(position).read(cx).panel_handles();
            for panel in panels {
                let target = zedx_default_dock_position(panel.persistent_name());
                if target != position && panel.position_is_valid(target, cx) {
                    if let Some(restore) = self.zedx_layout_restore.as_mut() {
                        restore.targets.push((panel.panel_id(), target));
                    }
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
            workspace.finish_zedx_layout_restore(window, cx);
        });
        let mut order = cx
            .try_global::<crate::zedx_rail_order::RailOrder>()
            .cloned()
            .unwrap_or_default();
        order.reset();
        cx.set_global(order);
        cx.notify();
    }

    fn finish_zedx_layout_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(restore) = self.zedx_layout_restore.as_ref() else {
            return;
        };
        if restore.targets.iter().any(|(panel_id, position)| {
            !self
                .dock_at_position(*position)
                .read(cx)
                .panel_handles()
                .iter()
                .any(|panel| panel.panel_id() == *panel_id)
        }) {
            return;
        }
        self.zedx_layout_restore = None;
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

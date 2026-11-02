use super::*;

pub(super) struct RailDropTarget {
    pub(super) dock: Entity<Dock>,
    pub(super) position: DockPosition,
    pub(super) name: String,
}

impl RailDropTarget {
    pub(super) fn reorder(
        &self,
        dragged: &str,
        bounds: gpui::Bounds<Pixels>,
        position: gpui::Point<Pixels>,
        window: &Window,
        cx: &mut App,
    ) {
        if dragged == self.name || !bounds.contains(&position) {
            return;
        }
        let visible = {
            let dock = self.dock.read(cx);
            let order = cx.try_global::<crate::zedx_rail_order::RailOrder>();
            let mut names = dock
                .panel_handles()
                .iter()
                .filter(|panel| {
                    panel.enabled(cx)
                        && panel.icon(window, cx).is_some()
                        && (matches!(panel.persistent_name(), "Project Panel" | "GitPanel")
                            || panel.icon_tooltip(window, cx).is_some())
                        && zedx_panel_is_pinned(order, self.position, panel.persistent_name())
                })
                .map(|panel| panel.persistent_name().to_string())
                .collect::<Vec<_>>();
            names.sort_by_key(|name| zedx_rail_sort_key(order, self.position, name));
            names
        };
        let Some(over) = visible.iter().position(|name| name == &self.name) else {
            return;
        };
        let Some(from) = visible.iter().position(|name| name == dragged) else {
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
        order.move_subset_to(self.position, &visible, dragged, target);
        cx.set_global(order);
        self.dock.update(cx, |_, cx| cx.notify());
    }
}

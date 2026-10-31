use super::*;

impl Workspace {
    pub(super) fn observe_zedx_compact_docks(&self, window: &mut Window, cx: &mut Context<Self>) {
        if !super::zedx_workbench::enabled() {
            return;
        }
        for (dock, other) in [
            (self.left_dock.clone(), self.right_dock.clone()),
            (self.right_dock.clone(), self.left_dock.clone()),
        ] {
            let mut previous = (dock.read(cx).is_open(), dock.read(cx).active_panel_index());
            cx.observe_in(&dock, window, move |workspace, dock, window, cx| {
                let state = (dock.read(cx).is_open(), dock.read(cx).active_panel_index());
                let opened = state.0 && state != previous;
                previous = state;
                if opened && window.viewport_size().width < px(560.0) {
                    other.update(cx, |dock, cx| dock.set_open(false, window, cx));
                    workspace.serialize_workspace(window, cx);
                }
            })
            .detach();
        }
        let mut was_compact = window.viewport_size().width < px(560.0);
        cx.observe_window_bounds(window, move |workspace, window, cx| {
            let compact = window.viewport_size().width < px(560.0);
            if compact && !was_compact {
                let right_focused = workspace
                    .right_dock
                    .read(cx)
                    .focus_handle(cx)
                    .contains_focused(window, cx);
                let other = if right_focused {
                    &workspace.left_dock
                } else {
                    &workspace.right_dock
                };
                other.update(cx, |dock, cx| dock.set_open(false, window, cx));
            }
            was_compact = compact;
        })
        .detach();
        cx.subscribe_in(
            &cx.entity(),
            window,
            |workspace, _, event: &Event, window, cx| {
                if matches!(event, Event::ActiveItemChanged)
                    && window.viewport_size().width < px(560.0)
                {
                    let left_focused = workspace
                        .left_dock
                        .read(cx)
                        .focus_handle(cx)
                        .contains_focused(window, cx);
                    workspace
                        .left_dock
                        .update(cx, |dock, cx| dock.set_open(false, window, cx));
                    if left_focused {
                        window.focus(&workspace.active_pane.focus_handle(cx), cx);
                    }
                    workspace.serialize_workspace(window, cx);
                }
            },
        )
        .detach();
    }
}

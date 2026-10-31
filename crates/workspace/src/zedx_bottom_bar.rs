use super::*;

impl Workspace {
    pub(super) fn render_zedx_bottom_bar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let dock = self.bottom_dock.read(cx);
        let clear_enabled = dock
            .active_panel()
            .is_some_and(|panel| matches!(panel.persistent_name(), "TerminalPanel" | "Run"));
        let focus = dock
            .active_panel()
            .map(|panel| panel.activation_focus_handle(cx));
        let tabs = dock.zedx_tabs(cx);
        h_flex()
            .h(px(28.0))
            .flex_none()
            .w_full()
            .gap_1()
            .px_1()
            .bg(cx.theme().colors().surface_background)
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .id("zedx-bottom-tabs")
                    .flex_1()
                    .min_w_0()
                    .overflow_x_scroll()
                    .children(tabs.into_iter().map(|(index, label, selected)| {
                        Button::new(("zedx-bottom-tab", index), label)
                            .label_size(LabelSize::Small)
                            .toggle_state(selected)
                            .on_click(cx.listener(move |workspace, _, window, cx| {
                                let focus = workspace.bottom_dock.update(cx, |dock, cx| {
                                    dock.activate_panel(index, window, cx);
                                    dock.set_open(true, window, cx);
                                    dock.active_panel()
                                        .map(|panel| panel.activation_focus_handle(cx))
                                });
                                if let Some(focus) = focus {
                                    window.focus(&focus, cx);
                                }
                                workspace.serialize_workspace(window, cx);
                            }))
                    })),
            )
            .child(
                ui::ButtonLike::new("zedx-clear-console")
                    .width(px(24.0))
                    .height(px(24.0).into())
                    .disabled(!clear_enabled)
                    .aria_label("Clear console")
                    .tooltip(ui::Tooltip::text("Clear console"))
                    .child(ui::zedx::icon(ui::zedx::Icon::Refresh, cx).size(px(16.0)))
                    .on_click(move |_, window, cx| {
                        if clear_enabled && let Some(focus) = &focus {
                            window.focus(focus, cx);
                            if let Some(action) = cx.build_action("terminal::Clear", None).log_err()
                            {
                                window.dispatch_action(action, cx);
                            }
                        }
                    }),
            )
            .child(
                ui::ButtonLike::new("zedx-hide-bottom")
                    .width(px(24.0))
                    .height(px(24.0).into())
                    .aria_label("Hide tool window")
                    .tooltip(ui::Tooltip::text("Hide tool window"))
                    .child(ui::zedx::icon(ui::zedx::Icon::Minus, cx).size(px(16.0)))
                    .on_click(cx.listener(|workspace, _, window, cx| {
                        workspace
                            .bottom_dock
                            .update(cx, |dock, cx| dock.set_open(false, window, cx));
                        window.focus(&workspace.focus_handle(cx), cx);
                        workspace.serialize_workspace(window, cx);
                    })),
            )
    }
}

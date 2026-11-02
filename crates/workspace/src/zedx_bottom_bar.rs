use super::*;
use ui::CountBadge;
fn shell_label(shell: &str) -> String {
    let name = std::path::Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shell);
    format!("Local · {name}")
}

impl Workspace {
    pub(super) fn render_zedx_bottom_bar(
        &self,
        window: &mut Window,
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
        let panels = dock.panel_handles();
        let active_label = dock.active_panel().map(|panel| panel.persistent_name());
        h_flex()
            .h(px(28.0))
            .flex_none()
            .w_full()
            .gap_1()
            .px_1()
            .bg(cx.theme().colors().surface_background)
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(h_flex().id("zedx-bottom-tabs").flex_1().min_w_0().children(
                tabs.into_iter().map(|(index, label, selected)| {
                    let count = panels
                        .get(index)
                        .and_then(|panel| panel.icon_label(window, cx))
                        .and_then(|count| count.parse::<usize>().ok());
                    let icon = match label {
                        "Terminal" => Some(ui::zedx::Icon::Terminal),
                        "Run" => Some(ui::zedx::Icon::Play),
                        "Problems" => Some(ui::zedx::Icon::Warning),
                        "Debug" => Some(ui::zedx::Icon::Debug),
                        _ => None,
                    };
                    div()
                        .debug_selector(move || format!("zedx-bottom-tab-{label}"))
                        .child(
                            ui::ButtonLike::new(("zedx-bottom-tab", index))
                                .height(px(24.0).into())
                                .toggle_state(selected)
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .px_2()
                                        .items_center()
                                        .when_some(icon, |row, icon| {
                                            row.child(ui::zedx::icon(icon, cx).size(px(14.0)))
                                        })
                                        .child(Label::new(label).size(LabelSize::XSmall))
                                        .when_some(count.filter(|_| !selected), |row, count| {
                                            row.child(
                                                div()
                                                    .debug_selector(move || {
                                                        format!("zedx-bottom-count-{label}")
                                                    })
                                                    .child(CountBadge::new(count)),
                                            )
                                        }),
                                )
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
                                })),
                        )
                }),
            ))
            .when(active_label == Some("TerminalPanel"), |bar| {
                bar.child(
                    div()
                        .debug_selector(|| "zedx-terminal-shell-label".into())
                        .child(
                            Label::new(shell_label(&util::get_system_shell()))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        ),
                )
            })
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
                div().debug_selector(|| "zedx-hide-bottom".into()).child(
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
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::shell_label;

    #[test]
    fn shell_label_uses_the_shell_program_name() {
        assert_eq!(shell_label("/bin/zsh"), "Local · zsh");
        assert_eq!(shell_label("/usr/local/bin/fish"), "Local · fish");
        assert_eq!(shell_label("pwsh"), "Local · pwsh");
    }
}

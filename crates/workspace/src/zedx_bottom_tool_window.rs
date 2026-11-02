use super::*;
fn shell_label(shell: &str) -> String {
    let name = std::path::Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shell);
    format!("Local · {name}")
}

impl Workspace {
    pub(super) fn render_zedx_bottom_tool_window_head(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let dock = self.bottom_dock.read(cx);
        let clear_enabled = dock
            .active_panel()
            .is_some_and(|panel| matches!(panel.persistent_name(), "TerminalPanel" | "Run"));
        let focus = dock
            .active_panel()
            .map(|panel| panel.activation_focus_handle(cx));
        let name = dock.active_panel()?.persistent_name();
        let (id, title, icon) = match name {
            "TerminalPanel" => ("terminal", "Terminal", ui::zedx::Icon::Terminal),
            "Run" => ("run", "Run", ui::zedx::Icon::Play),
            "Git history" => ("git", "Git", ui::zedx::Icon::Git),
            "DebugPanel" => ("debug", "Debug", ui::zedx::Icon::Debug),
            _ => return None,
        };
        Some(
            ui::zedx::tool_window_head(id, icon, title, cx)
                .when(name == "TerminalPanel", |bar| {
                    bar.child(
                        div()
                            .debug_selector(|| "zedx-terminal-shell-label".into())
                            .child(
                                Label::new(shell_label(&util::get_system_shell()))
                                    .size(ui::zedx::caption_size())
                                    .color(Color::Muted),
                            ),
                    )
                })
                .child(div().flex_1())
                .when(clear_enabled, |bar| {
                    bar.child(
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
                                    if let Some(action) =
                                        cx.build_action("terminal::Clear", None).log_err()
                                    {
                                        window.dispatch_action(action, cx);
                                    }
                                }
                            }),
                    )
                })
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
                                window.focus(&workspace.active_pane.focus_handle(cx), cx);
                                workspace.serialize_workspace(window, cx);
                            })),
                    ),
                )
                .into_any_element(),
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

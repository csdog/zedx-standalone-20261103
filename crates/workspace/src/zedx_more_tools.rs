use super::*;

impl Workspace {
    pub(super) fn render_zedx_more_tools(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.zedx_tool_window_entries(cx);
        let right_order = self.zedx_right_control_order(cx);
        ui::PopoverMenu::new("zedx-more-tools")
            .trigger_with_tooltip(
                ui::ButtonLike::new("zedx-more-tools")
                    .width(px(32.0))
                    .height(px(34.0).into())
                    .aria_label("Show or hide tool window buttons")
                    .tab_index(0isize)
                    .child(
                        div()
                            .debug_selector(|| "zedx-more-tools".into())
                            .child(ui::zedx::icon(ui::zedx::Icon::More, cx)),
                    ),
                ui::Tooltip::text("Show or hide tool window buttons"),
            )
            .menu(move |window, cx| {
                let entries = entries.clone();
                let right_order = right_order.clone();
                Some(ui::ContextMenu::build(
                    window,
                    cx,
                    move |mut menu, _, cx| {
                        menu = menu.keep_open_on_confirm(true);
                        for (position, title) in [
                            (DockPosition::Left, "Left"),
                            (DockPosition::Bottom, "Bottom"),
                            (DockPosition::Right, "Right"),
                        ] {
                            menu = menu.header(title);
                            if position == DockPosition::Right {
                                menu = visibility_entry(
                                    menu,
                                    "Notifications".into(),
                                    "Notifications".into(),
                                    true,
                                );
                                for name in &right_order {
                                    let label = match name.as_str() {
                                        "AgentPanel" => "AI Assistant",
                                        "Outline Panel" => "Outline",
                                        _ => "Database",
                                    };
                                    menu = visibility_entry(menu, name.clone(), label.into(), true);
                                }
                            }
                            for (_, name, _, _) in entries.iter().filter(|entry| {
                                entry.0 == position
                                    && !matches!(
                                        entry.1.as_str(),
                                        "AgentPanel" | "Outline Panel" | "Database"
                                    )
                            }) {
                                let label = match name.as_str() {
                                    "Project Panel" => "Project",
                                    "GitPanel" => "Commit",
                                    "TerminalPanel" => "Terminal",
                                    "DebugPanel" => "Debug",
                                    name => name,
                                };
                                let default =
                                    crate::dock::zedx_tool_window_bar::zedx_panel_default_pinned(
                                        cx.try_global::<crate::zedx_rail_order::RailOrder>(),
                                        position,
                                        name,
                                    );
                                menu = visibility_entry(menu, name.clone(), label.into(), default);
                            }
                        }
                        menu
                    },
                ))
            })
    }
}

fn visibility_entry(
    menu: ui::ContextMenu,
    name: String,
    label: String,
    default: bool,
) -> ui::ContextMenu {
    let render_name = name.clone();
    menu.custom_entry(
        move |_, cx| {
            let visible = crate::zedx_tool_visibility::is_visible(&render_name, default, cx);
            div()
                .debug_selector({
                    let label = label.clone();
                    move || format!("MENU_ITEM-{label}")
                })
                .w_full()
                .min_w(gpui::rems(14.0))
                .py_0p5()
                .child(
                    ui::Checkbox::new(
                        gpui::SharedString::from(format!("zedx-tool-visibility-{render_name}")),
                        ui::ToggleState::from(visible),
                    )
                    .solid_selected()
                    .label(label.clone())
                    .label_size(ui::zedx::label_size())
                    .label_color(Color::Default),
                )
                .into_any_element()
        },
        move |_, cx| crate::zedx_tool_visibility::toggle(&name, default, cx),
    )
}

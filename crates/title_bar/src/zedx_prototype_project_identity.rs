/// Project avatars use two letters: two words contribute one letter each,
/// while a single word contributes its first two letters (the prototype shows
/// `ZE` for `zedx`).
pub(super) fn initials_for(name: Option<&str>) -> String {
    let words = name
        .unwrap_or("Project")
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    match words.as_slice() {
        [] => "PR".to_string(),
        [word] => word.chars().take(2).flat_map(char::to_uppercase).collect(),
        [first, second, ..] => [first, second]
            .into_iter()
            .filter_map(|word| word.chars().next())
            .flat_map(char::to_uppercase)
            .collect(),
    }
}

use super::*;

impl TitleBar {
    pub(super) fn render_zedx_project_identity(
        &self,
        name: Option<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let compact = window.viewport_size().width <= px(760.0);
        let initials = initials_for(name.as_deref());
        let label = name.as_deref().unwrap_or("Projects");
        let label = util::truncate_and_trailoff(
            label,
            if compact || window.viewport_size().width < px(900.0) {
                8
            } else if window.viewport_size().width <= px(1100.0) {
                12
            } else {
                24
            },
        );
        let workspace = self.workspace.clone();
        let focus = workspace
            .upgrade()
            .map(|workspace| workspace.read(cx).focus_handle(cx))
            .unwrap_or_else(|| cx.focus_handle());
        let groups = self
            .multi_workspace
            .as_ref()
            .and_then(|multi| multi.upgrade())
            .map(|multi| multi.read(cx).project_group_keys())
            .unwrap_or_default();
        div()
            .debug_selector(|| "zedx-project-identity".into())
            .child(
                PopoverMenu::new("zedx-recent-projects")
                    .menu(move |window, cx| {
                        Some(recent_projects::RecentProjects::popover(
                            workspace.clone(),
                            groups.clone(),
                            None,
                            focus.clone(),
                            window,
                            cx,
                        ))
                    })
                    .trigger_with_tooltip(
                        ui::ButtonLike::new("zedx-project-switch")
                            .height(px(32.0).into())
                            .tab_index(0isize)
                            .aria_label("Recent Projects")
                            .child(
                                h_flex()
                                    .max_w(if compact {
                                        (window.viewport_size().width - px(250.0)).max(px(64.0))
                                    } else {
                                        px(330.0)
                                    })
                                    .min_w_0()
                                    .gap(px(8.0))
                                    .px(px(7.0))
                                    .child(
                                        h_flex()
                                            .debug_selector(|| "zedx-project-avatar".into())
                                            .size(px(23.0))
                                            .flex_none()
                                            .justify_center()
                                            .rounded(px(5.0))
                                            .bg(gpui::linear_gradient(
                                                135.0,
                                                gpui::linear_color_stop(gpui::rgb(0x8bb7a2), 0.0),
                                                gpui::linear_color_stop(gpui::rgb(0x557d83), 1.0),
                                            ))
                                            .child(
                                                Label::new(initials)
                                                    .size(LabelSize::Custom(gpui::rems(
                                                        11.0 / 13.0,
                                                    )))
                                                    .color(Color::Custom(
                                                        gpui::rgb(0xffffff).into(),
                                                    )),
                                            ),
                                    )
                                    .child(
                                        Label::new(label)
                                            .size(ui::zedx::label_size())
                                            .truncate()
                                            .weight(gpui::FontWeight::MEDIUM),
                                    )
                                    .child(
                                        Icon::new(IconName::ChevronDown)
                                            .size(IconSize::Custom(gpui::rems(12.0 / 16.0))),
                                    ),
                            ),
                        Tooltip::text("Recent Projects"),
                    )
                    .anchor(gpui::Anchor::TopLeft),
            )
    }
}

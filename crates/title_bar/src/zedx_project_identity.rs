use super::*;

impl TitleBar {
    pub(super) fn render_zedx_project_identity(
        &self,
        name: Option<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let initials: String = name
            .as_deref()
            .unwrap_or("Project")
            .split(|character: char| !character.is_alphanumeric())
            .filter_map(|word| word.chars().next())
            .take(2)
            .flat_map(char::to_uppercase)
            .collect();
        let name = name.map(|name| {
            if window.viewport_size().width < px(560.0) {
                util::truncate_and_trailoff(&name, 10).into()
            } else {
                name
            }
        });
        let workspace = self.workspace.clone();
        h_flex()
            .debug_selector(|| "zedx-project-identity".into())
            .gap_1()
            .when(window.viewport_size().width >= px(760.0), |row| {
                row.child(
                    ui::ButtonLike::new("zedx-main-menu")
                        .width(px(28.0))
                        .height(px(28.0).into())
                        .tab_index(0isize)
                        .aria_label("Main menu")
                        .tooltip(Tooltip::text("Main menu"))
                        .child(ui::zedx::icon(ui::zedx::Icon::Menu, cx))
                        .on_click(move |_, window, cx| {
                            if let Some(workspace) = workspace.upgrade() {
                                window.focus(&workspace.read(cx).focus_handle(cx), cx);
                            }
                            window.dispatch_action(
                                workspace::zedx_actions::SearchActions.boxed_clone(),
                                cx,
                            );
                        }),
                )
            })
            .child(
                h_flex()
                    .size(px(24.0))
                    .justify_center()
                    .rounded_md()
                    .bg(cx.theme().colors().element_selected)
                    .child(
                        Label::new(initials)
                            .size(LabelSize::XSmall)
                            .color(Color::Accent),
                    ),
            )
            .child(self.render_project_name(name, window, cx))
    }
}

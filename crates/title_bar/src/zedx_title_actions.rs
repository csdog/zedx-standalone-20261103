use super::*;
use ui::ButtonLike;
use workspace::zedx_actions::{SearchEverywhere, ToggleTheme};

impl TitleBar {
    pub(super) fn render_zedx_actions(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let compact = window.viewport_size().width < px(760.0);
        let button = |id, icon, label: &'static str, action: Box<dyn Action>| {
            let workspace = self.workspace.clone();
            ButtonLike::new(id)
                .width(px(28.0))
                .height(px(28.0).into())
                .aria_label(label)
                .tab_index(0isize)
                .child(ui::zedx::icon(icon, cx).size(px(16.0)))
                .tooltip(Tooltip::text(label))
                .on_click(move |_, window, cx| {
                    if let Some(workspace) = workspace.upgrade() {
                        window.focus(&workspace.read(cx).focus_handle(cx), cx);
                    }
                    window.dispatch_action(action.boxed_clone(), cx);
                })
        };
        let workspace = self.workspace.clone();
        h_flex()
            .debug_selector(|| "zedx-title-actions".into())
            .gap_1()
            .when(!compact, |row| {
                row.child(
                    Button::new("zedx-run-configuration", "Run configuration")
                        .label_size(LabelSize::Small)
                        .end_icon(Icon::new(IconName::ChevronDown).size(IconSize::XSmall))
                        .tooltip(Tooltip::text("Choose and run a project task"))
                        .on_click(move |_, window, cx| {
                            if let Some(workspace) = workspace.upgrade() {
                                window.focus(&workspace.read(cx).focus_handle(cx), cx);
                            }
                            window.dispatch_action(zed_actions::Spawn::modal().boxed_clone(), cx);
                        }),
                )
            })
            .child(button(
                "zedx-run",
                ui::zedx::Icon::Play,
                "Run last task",
                zed_actions::Rerun::default().boxed_clone(),
            ))
            .children(
                cx.build_action("debugger::Start", None)
                    .log_err()
                    .map(|action| {
                        button("zedx-debug", ui::zedx::Icon::Debug, "Debug project", action)
                    }),
            )
            .child(if compact {
                button(
                    "zedx-search",
                    ui::zedx::Icon::Search,
                    "Search Everywhere · Shift Shift",
                    SearchEverywhere.boxed_clone(),
                )
                .into_any_element()
            } else {
                let action = SearchEverywhere.boxed_clone();
                let workspace = self.workspace.clone();
                div()
                    .debug_selector(|| "zedx-search-label".into())
                    .child(
                        ButtonLike::new("zedx-search")
                            .height(px(28.0).into())
                            .aria_label("Search Everywhere")
                            .tooltip(Tooltip::text("Search Everywhere · Shift Shift"))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .px_2()
                                    .items_center()
                                    .child(
                                        ui::zedx::icon(ui::zedx::Icon::Search, cx).size(px(16.0)),
                                    )
                                    .child(Label::new("Search Everywhere").size(LabelSize::XSmall))
                                    .child(
                                        Label::new("⇧ ⇧")
                                            .size(LabelSize::XSmall)
                                            .color(Color::Muted),
                                    ),
                            )
                            .on_click(move |_, window, cx| {
                                if let Some(workspace) = workspace.upgrade() {
                                    window.focus(&workspace.read(cx).focus_handle(cx), cx);
                                }
                                window.dispatch_action(action.boxed_clone(), cx);
                            }),
                    )
                    .into_any_element()
            })
            .child(button(
                "zedx-theme",
                ui::zedx::Icon::Sun,
                "Switch light / dark theme",
                ToggleTheme.boxed_clone(),
            ))
    }
}

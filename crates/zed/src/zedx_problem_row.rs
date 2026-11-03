use super::*;

impl Problems {
    pub(super) fn render_problem_row(
        &self,
        index: usize,
        row: Problem,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let summary = row.message.split_whitespace().collect::<Vec<_>>().join(" ");
        h_flex()
            .id(("zedx-problem-row", index))
            .debug_selector(move || format!("zedx-problem-row-{index}"))
            .w_full()
            .h(gpui::rems(2.25))
            .min_h(gpui::rems(2.25))
            .flex_none()
            .px_2()
            .gap_2()
            .overflow_hidden()
            .cursor_pointer()
            .hover(|style| style.bg(cx.theme().colors().ghost_element_hover))
            .child(
                Icon::new(if row.error {
                    IconName::XCircle
                } else {
                    IconName::Warning
                })
                .size(IconSize::Small)
                .color(if row.error {
                    Color::Error
                } else {
                    Color::Warning
                }),
            )
            .child(
                div()
                    .debug_selector(move || format!("zedx-problem-message-{index}"))
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .child(
                        Label::new(summary)
                            .size(ui::zedx::label_size())
                            .line_height_style(ui::LineHeightStyle::UiLabel)
                            .truncate(),
                    ),
            )
            .child(
                div()
                    .debug_selector(move || format!("zedx-problem-location-{index}"))
                    .w(gpui::rems(18.0))
                    .max_w(gpui::relative(0.35))
                    .flex_shrink_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(
                        Label::new(row.location.clone())
                            .size(ui::zedx::caption_size())
                            .line_height_style(ui::LineHeightStyle::UiLabel)
                            .color(Color::Muted)
                            .truncate_start(),
                    ),
            )
            .tooltip(Tooltip::text(format!(
                "{}\n{}",
                row.message,
                row.path.path.as_unix_str()
            )))
            .on_click(cx.listener(move |this, _, window, cx| this.open(row.clone(), window, cx)))
            .into_any_element()
    }
}

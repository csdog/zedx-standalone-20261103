use super::*;

impl Render for Find {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scope = if self.include_ignored {
            "All Places"
        } else {
            "Project"
        };
        let files = self
            .entries
            .iter()
            .filter(|entry| matches!(entry, Entry::File { .. }))
            .count();
        let file_label = if files == 1 { "file" } else { "files" };
        let summary = if self.searching {
            format!("In {scope} · Searching…")
        } else if self.rows.len() == 1000 {
            format!("In {scope} · First 1000 matches in {files} {file_label}")
        } else {
            format!(
                "In {scope} · {} matches in {files} {file_label}",
                self.rows.len()
            )
        };
        let colors = cx.theme().colors();
        let focused = self.query.focus_handle(cx).contains_focused(window, cx);
        let input_background = colors.editor_background;
        let input_border = if focused {
            colors.border_focused
        } else {
            colors.border
        };
        v_flex()
            .debug_selector(|| "zedx-find-panel".into())
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .key_context("ZedXFind")
            .on_action(cx.listener(|this, _: &menu::SelectNext, _, cx| this.select(false, cx)))
            .on_action(cx.listener(|this, _: &menu::SelectPrevious, _, cx| this.select(true, cx)))
            .on_action(cx.listener(|this, _: &menu::Confirm, window, cx| {
                if let Some(row) = this.rows.get(this.selected).cloned() {
                    this.open(row, window, cx);
                }
            }))
            .child(
                ui::zedx::tool_window_head("find", ui::zedx::Icon::FindTool, "Find", cx)
                    .child(
                        Button::new("zedx-find-scope", scope)
                            .label_size(ui::zedx::caption_size())
                            .tooltip(Tooltip::text("Toggle ignored files"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.include_ignored = !this.include_ignored;
                                this.refresh(true, window, cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div().debug_selector(|| "zedx-find-hide".into()).child(
                            ui::zedx::tool_button(
                                "zedx-find-hide",
                                ui::zedx::Icon::Minus,
                                "Hide Find",
                                cx,
                            )
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Close))),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .px(px(14.0))
                    .py(px(10.0))
                    .child(
                        h_flex()
                            .debug_selector(|| "zedx-find-field".into())
                            .h(px(34.0))
                            .flex_none()
                            .min_w_0()
                            .mt(px(2.0))
                            .mb(px(12.0))
                            .px(px(10.0))
                            .rounded(px(5.0))
                            .border_1()
                            .border_color(input_border)
                            .bg(input_background)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .child(self.query.clone()),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "zedx-find-clear-button".into())
                                    .child(
                                        ui::zedx::tool_button(
                                            "zedx-find-clear",
                                            ui::zedx::Icon::Close,
                                            "Clear query",
                                            cx,
                                        )
                                        .on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.query.update(cx, |editor, cx| {
                                                    editor.set_text("", window, cx)
                                                });
                                                window.focus(&this.query.focus_handle(cx), cx);
                                            }),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "zedx-find-summary".into())
                            .flex_none()
                            .px(px(2.0))
                            .pb(px(8.0))
                            .child(
                                Label::new(summary)
                                    .size(ui::zedx::caption_size())
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .when_some(self.error.clone(), |body, error| {
                        body.child(Label::new(error).color(Color::Warning))
                    })
                    .when(self.rows.is_empty() && !self.searching, |body| {
                        body.child(
                            Label::new(if self.last_query.is_empty() {
                                "Search text across project files"
                            } else {
                                "No matches"
                            })
                            .size(ui::zedx::label_size())
                            .color(Color::Muted),
                        )
                    })
                    .child(
                        uniform_list(
                            "zedx-find-results",
                            self.entries.len(),
                            cx.processor(|this, range: Range<usize>, _, cx| {
                                range
                                    .filter_map(|display_index| {
                                        match this.entries.get(display_index)?.clone() {
                                            Entry::File { path, count } => Some(
                                                h_flex()
                                                    .h(px(28.0))
                                                    .px_2()
                                                    .gap_1()
                                                    .min_w_0()
                                                    .child(
                                                        Icon::new(IconName::File)
                                                            .size(IconSize::Small)
                                                            .color(Color::Muted),
                                                    )
                                                    .child(
                                                        Label::new(path)
                                                            .size(ui::zedx::caption_size())
                                                            .truncate(),
                                                    )
                                                    .child(
                                                        Label::new(count.to_string())
                                                            .size(ui::zedx::caption_size())
                                                            .color(Color::Muted),
                                                    )
                                                    .into_any_element(),
                                            ),
                                            Entry::Match(index) => {
                                                let row = this.rows.get(index)?.clone();
                                                Some(
                                                    ListItem::new(display_index)
                                                        .spacing(ListItemSpacing::Dense)
                                                        .height(px(28.0))
                                                        .toggle(index == this.selected)
                                                        .child(
                                                            h_flex()
                                                                .min_w_0()
                                                                .gap_2()
                                                                .pl(px(12.0))
                                                                .child(
                                                                    Label::new(
                                                                        (row.point.row + 1)
                                                                            .to_string(),
                                                                    )
                                                                    .size(ui::zedx::caption_size())
                                                                    .buffer_font(cx)
                                                                    .color(Color::Muted),
                                                                )
                                                                .child(
                                                                    HighlightedLabel::from_ranges(
                                                                        row.text.clone(),
                                                                        vec![row.highlight.clone()],
                                                                    )
                                                                    .size(ui::zedx::caption_size())
                                                                    .buffer_font(cx)
                                                                    .truncate(),
                                                                ),
                                                        )
                                                        .tooltip(Tooltip::text(
                                                            row.location.clone(),
                                                        ))
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                this.selected = index;
                                                                cx.notify();
                                                                this.open(row.clone(), window, cx);
                                                            },
                                                        ))
                                                        .into_any_element(),
                                                )
                                            }
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }),
                        )
                        .debug_selector(|| "zedx-find-results".into())
                        .track_scroll(&self.scroll)
                        .flex_1(),
                    ),
            )
    }
}

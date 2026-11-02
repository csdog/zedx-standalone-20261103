use super::*;

actions!(zedx_git, [ToggleZedxBranches, RefreshZedxBranches]);

impl GitGraph {
    pub fn use_zedx_toolbar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.zedx_toolbar.is_none() {
            self.commit_details_split_state.update(cx, |state, _| {
                state.left_ratio = 0.74;
                state.visible_left_ratio = 0.74;
            });
        }
        self.zedx_toolbar = Some(window.viewport_size().width);
        self.initialize_zedx_filter(cx);
        self.search_state.editor.update(cx, |editor, cx| {
            editor.set_placeholder_text("Text or hash", window, cx)
        });
    }

    pub fn set_zedx_toolbar_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        if self.zedx_toolbar != Some(width) {
            self.zedx_toolbar = Some(width);
            cx.notify();
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn zedx_search_text_for_test(&self, cx: &App) -> String {
        self.search_state.editor.read(cx).text(cx)
    }

    pub fn set_zedx_log_source(&mut self, source: LogSource, cx: &mut Context<Self>) {
        if self.log_source == source {
            return;
        }
        self.log_source = source;
        self.selected_entry_idx = None;
        self.hovered_entry_idx = None;
        self.pending_select_sha = None;
        self._commit_diff_task = None;
        self._selected_commit_message_task = None;
        self.selected_commit_diff = None;
        self.selected_commit_diff_stats = None;
        self.selected_commit_message = None;
        self.invalidate_state(cx);
    }

    pub(super) fn render_zedx_toolbar(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let branch = match &self.log_source {
            LogSource::Branch(name) => name
                .as_str()
                .strip_prefix("refs/heads/")
                .or_else(|| name.as_str().strip_prefix("refs/remotes/"))
                .or_else(|| name.as_str().strip_prefix("refs/tags/"))
                .unwrap_or(name.as_str()),
            _ => "All",
        };
        let focus = self.focus_handle.clone();
        h_flex()
            .key_context("GitGraphSearchBar")
            .debug_selector(|| "zedx-git-filters".into())
            .w_full()
            .h(px(32.0))
            .flex_none()
            .gap_1()
            .px_1()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                ui::zedx::tool_button(
                    "zedx-git-show-branches",
                    ui::zedx::Icon::Chevron,
                    "Show / hide branches",
                    cx,
                )
                .on_click(move |_, window, cx| {
                    window.focus(&focus, cx);
                    window.dispatch_action(ToggleZedxBranches.boxed_clone(), cx);
                }),
            )
            .child(
                h_flex()
                    .w(px(170.0))
                    .min_w(px(80.0))
                    .h(px(24.0))
                    .px_1()
                    .border_1()
                    .rounded_sm()
                    .border_color(cx.theme().colors().border_variant)
                    .bg(cx.theme().colors().editor_background)
                    .debug_selector(|| "zedx-git-log-query".into())
                    .on_action(cx.listener(Self::confirm_search))
                    .child(self.search_state.editor.clone()),
            )
            .when(
                window.viewport_size().width >= px(760.0)
                    && self.zedx_toolbar.is_some_and(|width| width >= px(320.0)),
                |row| {
                    row.child(
                        div()
                            .debug_selector(|| "zedx-git-branch-filter".into())
                            .max_w(px(170.0))
                            .overflow_hidden()
                            .px_1()
                            .py_0p5()
                            .rounded_sm()
                            .bg(cx.theme().colors().element_selected)
                            .child(
                                Label::new(format!("Branch: {branch}"))
                                    .size(ui::zedx::caption_size())
                                    .truncate(),
                            ),
                    )
                },
            )
            .when(
                self.zedx_toolbar.is_some_and(|width| width >= px(620.0)),
                |row| {
                    row.children(["User", "Date", "Paths"].into_iter().map(|label| {
                        div().px_1().child(
                            Label::new(label)
                                .size(ui::zedx::caption_size())
                                .color(Color::Muted),
                        )
                    }))
                },
            )
            .child(div().flex_1())
            .child(
                ui::zedx::tool_button(
                    "zedx-git-refresh",
                    ui::zedx::Icon::Refresh,
                    "Refresh branches",
                    cx,
                )
                .on_click({
                    let focus = self.focus_handle.clone();
                    move |_, window, cx| {
                        window.focus(&focus, cx);
                        window.dispatch_action(RefreshZedxBranches.boxed_clone(), cx);
                    }
                }),
            )
            .child(
                ui::zedx::tool_button(
                    "zedx-git-view-options",
                    ui::zedx::Icon::More,
                    "View options",
                    cx,
                )
                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                    this.deploy_header_context_menu(event.position(), window, cx)
                })),
            )
            .into_any_element()
    }
}

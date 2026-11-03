use super::*;

pub(super) struct MessageHistory {
    panel: WeakEntity<GitPanel>,
    repository: Entity<Repository>,
    focus: FocusHandle,
    _subscriptions: [Subscription; 2],
}
impl MessageHistory {
    pub(super) fn new(
        panel: WeakEntity<GitPanel>,
        repository: Entity<Repository>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions = [
            cx.observe(&repository, |_, _, cx| cx.notify()),
            cx.subscribe(&repository, |_, _, _: &RepositoryEvent, cx| cx.notify()),
        ];
        Self {
            panel,
            repository,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }
}
impl Focusable for MessageHistory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl EventEmitter<DismissEvent> for MessageHistory {}
impl Render for MessageHistory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = GitPanel::commit_history_log_source(&self.repository, cx);
        let (entries, loading, error) = if let Some(source) = source {
            self.repository.update(cx, |repository, cx| {
                let response = repository.graph_data(source, LogOrder::DateOrder, 0..20, cx);
                let entries = response
                    .commits
                    .iter()
                    .take(20)
                    .map(|commit| commit.sha)
                    .collect::<Vec<_>>();
                (entries, response.is_loading, response.error)
            })
        } else {
            (Vec::new(), false, None)
        };
        let data = self.repository.update(cx, |repository, cx| {
            entries
                .iter()
                .map(|sha| match repository.fetch_commit_data(*sha, false, cx) {
                    CommitDataState::Loaded(data) => Some(data.clone()),
                    CommitDataState::Loading(_) => None,
                })
                .collect::<Vec<_>>()
        });
        let repository_id = self.repository.entity_id();
        v_flex()
            .debug_selector(|| "zedx-commit-history-popover".into())
            .track_focus(&self.focus)
            .w(px(280.0).min((window.viewport_size().width - px(24.0)).max(px(1.0))))
            .p(px(8.0))
            .gap(px(6.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().elevated_surface_background)
            .shadow_lg()
            .on_action(cx.listener(|_, _: &menu::Cancel, _, cx| cx.emit(DismissEvent)))
            .child(
                Label::new("Commit Message History")
                    .size(ui::zedx::caption_size())
                    .color(Color::Muted),
            )
            .when_some(error, |view, error| {
                view.child(
                    Label::new(error)
                        .size(ui::zedx::caption_size())
                        .color(Color::Error)
                        .truncate(),
                )
            })
            .when(entries.is_empty(), |view| {
                view.child(
                    Label::new(if loading {
                        "Loading commit messages…"
                    } else {
                        "No previous commit messages"
                    })
                    .size(ui::zedx::caption_size())
                    .color(Color::Muted),
                )
            })
            .child(
                v_flex()
                    .id("zedx-commit-history-list")
                    .max_h(px(260.0).min((window.viewport_size().height - px(90.0)).max(px(32.0))))
                    .overflow_y_scroll()
                    .children(data.into_iter().enumerate().map(|(index, data)| {
                        let title = data.as_ref().map_or_else(
                            || "Loading commit message…".to_string(),
                            |data| data.subject.to_string(),
                        );
                        let tooltip = data
                            .as_ref()
                            .map_or_else(|| title.clone(), |data| data.message.to_string());
                        div()
                            .debug_selector(move || format!("zedx-commit-history-{index}"))
                            .child(
                                ButtonLike::new(("zedx-commit-history-entry", index))
                                    .width(gpui::relative(1.0))
                                    .height(px(28.0).into())
                                    .size(ButtonSize::None)
                                    .disabled(data.is_none())
                                    .aria_label(title.clone())
                                    .tab_index(0isize)
                                    .child(
                                        Label::new(title).size(ui::zedx::caption_size()).truncate(),
                                    )
                                    .tooltip(Tooltip::text(tooltip))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if let Some(data) = &data {
                                            this.panel
                                                .update(cx, |panel, cx| {
                                                    if panel.pending_commit.is_none()
                                                        && panel
                                                            .active_repository
                                                            .as_ref()
                                                            .is_some_and(|repo| {
                                                                repo.entity_id() == repository_id
                                                            })
                                                    {
                                                        panel.commit_editor.update(
                                                            cx,
                                                            |editor, cx| {
                                                                editor.set_text(
                                                                    data.message.clone(),
                                                                    window,
                                                                    cx,
                                                                )
                                                            },
                                                        );
                                                        panel
                                                            .zedx_commit
                                                            .composer
                                                            .inspected_repository = None;
                                                        window.focus(
                                                            &panel.commit_editor.focus_handle(cx),
                                                            cx,
                                                        );
                                                    }
                                                })
                                                .log_err();
                                            cx.emit(DismissEvent);
                                        }
                                    })),
                            )
                    })),
            )
    }
}

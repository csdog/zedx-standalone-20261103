use super::*;

#[path = "zedx_commit_message_history.rs"]
mod history;

#[derive(Default)]
pub(super) struct ComposerState {
    skip_diagnostics: bool,
    push_by_default: bool,
    inspected_repository: Option<gpui::EntityId>,
    push_after_commit: Option<PushAfterCommit>,
    locked: bool,
}

struct PushAfterCommit {
    repository: gpui::EntityId,
    branch: Option<SharedString>,
}

impl GitPanel {
    fn zedx_message_error(&self, cx: &App) -> Option<&'static str> {
        if self.commit_editor.read(cx).text(cx).trim().is_empty() {
            return Some("Specify commit message");
        }
        if !self.has_staged_changes() && !self.amend_pending {
            return Some("Select files to commit");
        }
        if !self.zedx_commit.composer.skip_diagnostics
            && let Some(repository) = self.active_repository.as_ref()
        {
            let repository = repository.read(cx);
            let project = self.project.read(cx);
            let has_errors = self
                .change_entries_by_path()
                .filter(|entry| {
                    Self::stage_status_for_entry(entry, repository) != StageStatus::Unstaged
                })
                .filter_map(|entry| repository.repo_path_to_project_path(&entry.repo_path, cx))
                .any(|path| project.diagnostic_summary_for_path(&path, cx).error_count > 0);
            if has_errors {
                return Some("Resolve code errors in included files before committing");
            }
        }
        None
    }

    pub(super) fn submit_zedx_commit(
        &mut self,
        push: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_commit.is_some() || self.modal_open {
            return;
        }
        self.commit_editor_collapsed = false;
        self.zedx_commit.composer.inspected_repository =
            self.active_repository.as_ref().map(Entity::entity_id);
        if self.zedx_message_error(cx).is_some() {
            window.focus(&self.commit_editor.focus_handle(cx), cx);
            cx.notify();
            return;
        }
        if !self.configure_commit_button(cx).0 {
            return;
        }
        let push = push || self.zedx_commit.composer.push_by_default;
        if push && !self.zedx_can_push(cx) {
            return;
        }
        if push && let Some(repository) = &self.active_repository {
            self.zedx_commit.composer.push_after_commit = Some(PushAfterCommit {
                repository: repository.entity_id(),
                branch: repository
                    .read(cx)
                    .branch
                    .as_ref()
                    .map(|branch| branch.name().into()),
            });
        }
        self.commit_changes(self.commit_options(), window, cx);
        if self.pending_commit.is_some() {
            self.commit_editor
                .update(cx, |editor, _| editor.set_read_only(true));
            self.zedx_commit.composer.locked = true;
        } else {
            self.zedx_commit.composer.push_after_commit = None;
        }
        cx.notify();
    }

    pub(crate) fn finish_zedx_commit(
        &mut self,
        succeeded: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.zedx_commit.composer.locked {
            self.commit_editor
                .update(cx, |editor, _| editor.set_read_only(false));
            self.zedx_commit.composer.locked = false;
            cx.notify();
        }
        let Some(intent) = self.zedx_commit.composer.push_after_commit.take() else {
            return;
        };
        if !succeeded {
            return;
        }
        let same_target = self.active_repository.as_ref().is_some_and(|repository| {
            repository.entity_id() == intent.repository
                && repository
                    .read(cx)
                    .branch
                    .as_ref()
                    .map(|branch| SharedString::from(branch.name()))
                    == intent.branch
        });
        if same_target {
            self.push(false, true, window, cx);
        } else {
            self.workspace
                .update(cx, |workspace, cx| {
                    workspace.show_toast(workspace::Toast::new(
                    NotificationId::unique::<PushAfterCommit>(),
                    "Commit succeeded. Return to its original repository and branch to push.",
                ).autohide(), cx);
                })
                .log_err();
        }
    }

    fn toggle_zedx_amend(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let head_message = self.head_commit(cx).map(|commit| commit.message);
        self.set_amend_pending(!self.amend_pending, cx);
        if self.amend_pending
            && let Some(message) = head_message
        {
            self.commit_editor
                .update(cx, |editor, cx| editor.set_text(message, window, cx));
        }
    }

    fn zedx_can_push(&self, cx: &App) -> bool {
        self.can_push_and_pull(cx)
            && self.pending_remote_operation.is_none()
            && self
                .active_repository
                .as_ref()
                .is_some_and(|repository| repository.read(cx).branch.is_some())
    }

    fn zedx_commit_submit_button(
        &self,
        id: &'static str,
        label: &'static str,
        primary: bool,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let light = cx.theme().appearance().is_light();
        let background = if primary {
            if enabled {
                gpui::rgb(0x4a6fbf)
            } else if light {
                gpui::rgb(0xe0e5ed)
            } else {
                gpui::rgb(0x333a48)
            }
        } else if light {
            gpui::rgb(0xffffff)
        } else {
            gpui::rgb(0x2a2d34)
        };
        let push = !primary;
        div()
            .debug_selector(move || id.into())
            .w_full()
            .h(px(28.0))
            .rounded(px(4.0))
            .overflow_hidden()
            .bg(background)
            .border_1()
            .border_color(if primary {
                background.into()
            } else {
                cx.theme().colors().border
            })
            .when(enabled, |button| {
                button.hover(move |style| {
                    style.bg(if primary {
                        gpui::rgb(0x5a80d0)
                    } else if light {
                        gpui::rgb(0xf0f3f8)
                    } else {
                        gpui::rgb(0x343840)
                    })
                })
            })
            .child(
                ButtonLike::new(id)
                    .width(gpui::relative(1.0))
                    .height(px(28.0).into())
                    .size(ButtonSize::None)
                    .style(ButtonStyle::Transparent)
                    .disabled(!enabled)
                    .aria_label(label)
                    .tab_index(0isize)
                    .child(
                        Label::new(label)
                            .size(ui::zedx::caption_size())
                            .truncate()
                            .color(if !enabled {
                                Color::Disabled
                            } else if primary {
                                Color::Custom(gpui::rgb(0xffffff).into())
                            } else {
                                Color::Default
                            }),
                    )
                    .tooltip(Tooltip::text(label))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.submit_zedx_commit(push, window, cx)
                    })),
            )
    }

    pub(super) fn render_zedx_composer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let placeholder = self
            .commit_editor
            .update(cx, |editor, cx| editor.placeholder_text(cx));
        if placeholder.as_deref() != Some("Commit Message") {
            self.commit_editor.update(cx, |editor, cx| {
                editor.set_placeholder_text("Commit Message", window, cx)
            });
        }
        let focus = self.commit_editor.focus_handle(cx);
        let focused = focus.contains_focused(window, cx);
        let busy = self.pending_commit.is_some();
        let has_head = self.head_commit(cx).is_some();
        let included = self.has_staged_changes() || self.amend_pending;
        let enabled = included
            && !busy
            && !self.modal_open
            && !self.has_unstaged_conflicts()
            && self.has_write_access(cx)
            && !self.is_generating_commit_message();
        let message = self.commit_editor.read(cx).text(cx);
        let limit = GitPanelSettings::get_global(cx).commit_title_max_length;
        let inspection = self
            .active_repository
            .as_ref()
            .filter(|repo| Some(repo.entity_id()) == self.zedx_commit.composer.inspected_repository)
            .and_then(|_| self.zedx_message_error(cx));
        let long_title = limit > 0
            && message
                .lines()
                .next()
                .is_some_and(|title| commit_title_exceeds_limit(title, limit));
        let alert = inspection
            .map(str::to_string)
            .or_else(|| long_title.then(|| format!("Commit title exceeds {limit} characters")));
        let font_size = ThemeSettings::get_global(cx).git_commit_buffer_font_size(cx);
        let mut editor_style = git_commit_editor_style(font_size, cx);
        editor_style.text.line_height = (font_size * 1.6).into();
        let editor_background = editor_style.background;
        let short = self.zedx_commit_message_height(window) < px(100.0);
        let primary_label = if busy {
            "Committing…"
        } else if self.zedx_commit.composer.push_by_default {
            "Commit and Push…"
        } else {
            "Commit"
        };
        let panel = cx.weak_entity();
        let history_repository = self.active_repository.clone();
        v_flex()
            .debug_selector(|| "zedx-commit-composer".into())
            .size_full()
            .min_h_0()
            .on_action(cx.listener(|this, _: &git::Commit, window, cx| {
                this.submit_zedx_commit(false, window, cx);
            }))
            .when(!short, |composer| {
                composer.child(
                    h_flex()
                        .debug_selector(|| "zedx-commit-amend-row".into())
                        .w_full()
                        .h(px(30.0))
                        .flex_none()
                        .px(px(8.0))
                        .gap(px(6.0))
                        .child(
                            div().debug_selector(|| "zedx-commit-amend".into()).child(
                                Checkbox::new("zedx-commit-amend", self.amend_pending.into())
                                    .label("Amend")
                                    .label_size(ui::zedx::caption_size())
                                    .disabled(!has_head || busy || !self.has_write_access(cx))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.toggle_zedx_amend(window, cx)
                                    })),
                            ),
                        )
                        .child(
                            div().flex_1().min_w_0().child(
                                Label::new("last commit")
                                    .size(ui::zedx::caption_size())
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                        )
                        .child(
                            div().debug_selector(|| "zedx-commit-history".into()).child(
                                PopoverMenu::new("zedx-commit-history-menu")
                                    .trigger_with_tooltip(
                                        ButtonLike::new("zedx-commit-history")
                                            .width(px(22.0))
                                            .height(px(22.0).into())
                                            .size(ButtonSize::None)
                                            .disabled(busy)
                                            .aria_label("Commit Message History")
                                            .tab_index(0isize)
                                            .child(
                                                ui::zedx::icon(ui::zedx::Icon::History, cx)
                                                    .size(px(13.0)),
                                            ),
                                        Tooltip::text("Commit Message History"),
                                    )
                                    .anchor(Anchor::BottomRight)
                                    .menu(move |window, cx| {
                                        history_repository.clone().map(|repository| {
                                            cx.new(|cx| {
                                                history::MessageHistory::new(
                                                    panel.clone(),
                                                    repository,
                                                    window,
                                                    cx,
                                                )
                                            })
                                        })
                                    }),
                            ),
                        ),
                )
            })
            .when(!self.commit_editor_collapsed, |composer| {
                composer.child(
                    div()
                        .id("zedx-commit-message-input")
                        .debug_selector(|| "zedx-commit-message-input".into())
                        .flex_1()
                        .min_h_0()
                        .mx(px(8.0))
                        .px(px(8.0))
                        .py(px(6.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(if focused {
                            cx.theme().colors().border_focused
                        } else {
                            cx.theme().colors().border
                        })
                        .bg(editor_background)
                        .overflow_hidden()
                        .cursor_text()
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.commit_editor.focus_handle(cx), cx)
                        }))
                        .child(EditorElement::new(&self.commit_editor, editor_style)),
                )
            })
            .when(self.commit_editor_collapsed, |composer| {
                composer.child(div().flex_1())
            })
            .when_some(alert, |composer, alert| {
                composer.child(
                    div()
                        .debug_selector(|| "zedx-commit-inspect".into())
                        .flex_none()
                        .px(px(10.0))
                        .pt(px(2.0))
                        .child(
                            Label::new(alert)
                                .size(ui::zedx::caption_size())
                                .color(if inspection.is_some() {
                                    Color::Error
                                } else {
                                    Color::Warning
                                })
                                .truncate(),
                        ),
                )
            })
            .child(
                h_flex()
                    .debug_selector(|| "zedx-commit-message-actions".into())
                    .w_full()
                    .flex_none()
                    .p(px(if short { 4.0 } else { 8.0 }))
                    .gap(px(6.0))
                    .child(
                        div()
                            .w(px(if self.zedx_commit.composer.push_by_default {
                                104.0
                            } else {
                                66.0
                            }))
                            .flex_none()
                            .child(self.zedx_commit_submit_button(
                                "zedx-commit-submit",
                                primary_label,
                                true,
                                enabled
                                    && (!self.zedx_commit.composer.push_by_default
                                        || self.zedx_can_push(cx)),
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(self.zedx_commit_submit_button(
                                "zedx-commit-push",
                                "Commit and Push…",
                                false,
                                enabled && self.zedx_can_push(cx),
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .debug_selector(|| "zedx-commit-options".into())
                            .flex_none()
                            .child(
                                PopoverMenu::new("zedx-commit-options-menu")
                                    .trigger_with_tooltip(
                                        ButtonLike::new("zedx-commit-options")
                                            .width(px(24.0))
                                            .height(px(24.0).into())
                                            .size(ButtonSize::None)
                                            .aria_label("Commit Options")
                                            .tab_index(0isize)
                                            .child(
                                                ui::zedx::icon(ui::zedx::Icon::Settings, cx)
                                                    .size(px(14.0)),
                                            ),
                                        Tooltip::text("Commit Options"),
                                    )
                                    .anchor(Anchor::BottomRight)
                                    .menu({
                                        let panel = cx.entity();
                                        move |_, cx| {
                                            Some(cx.new(|cx| {
                                                CommitOptionsPopover::new(panel.clone(), cx)
                                            }))
                                        }
                                    }),
                            ),
                    ),
            )
    }
}

struct CommitOptionsPopover {
    panel: Entity<GitPanel>,
    focus: FocusHandle,
    _subscription: Subscription,
}
impl CommitOptionsPopover {
    fn new(panel: Entity<GitPanel>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&panel, |_, _, cx| cx.notify());
        Self {
            panel,
            focus: cx.focus_handle(),
            _subscription: subscription,
        }
    }
}
impl Focusable for CommitOptionsPopover {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl EventEmitter<DismissEvent> for CommitOptionsPopover {}
impl Render for CommitOptionsPopover {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = self.panel.read(cx);
        let busy = panel.pending_commit.is_some();
        let analysis = !panel.zedx_commit.composer.skip_diagnostics;
        let signoff = panel.signoff_enabled;
        let push = panel.zedx_commit.composer.push_by_default;
        let analysis_panel = self.panel.clone();
        let signoff_panel = self.panel.clone();
        let push_panel = self.panel.clone();
        v_flex()
            .debug_selector(|| "zedx-commit-options-popover".into())
            .track_focus(&self.focus)
            .w(px(220.0))
            .p(px(8.0))
            .gap(px(8.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().elevated_surface_background)
            .shadow_lg()
            .on_action(cx.listener(|_, _: &menu::Cancel, _, cx| cx.emit(DismissEvent)))
            .child(
                Label::new("Commit Options")
                    .size(ui::zedx::caption_size())
                    .color(Color::Muted),
            )
            .child(
                div()
                    .debug_selector(|| "zedx-option-analysis".into())
                    .child(
                        Checkbox::new("zedx-option-analysis", analysis.into())
                            .label("Check code diagnostics")
                            .label_size(ui::zedx::caption_size())
                            .disabled(busy)
                            .tooltip(Tooltip::text(
                                "Check current diagnostics for included files before committing",
                            ))
                            .on_click(move |_, _, cx| {
                                analysis_panel.update(cx, |panel, cx| {
                                    panel.zedx_commit.composer.skip_diagnostics =
                                        !panel.zedx_commit.composer.skip_diagnostics;
                                    cx.notify();
                                })
                            }),
                    ),
            )
            .child(
                div().debug_selector(|| "zedx-option-signoff".into()).child(
                    Checkbox::new("zedx-option-signoff", signoff.into())
                        .label("Sign-off commit")
                        .label_size(ui::zedx::caption_size())
                        .disabled(busy)
                        .on_click(move |_, _, cx| {
                            signoff_panel.update(cx, |panel, cx| {
                                panel.set_signoff_enabled(!panel.signoff_enabled, cx)
                            })
                        }),
                ),
            )
            .child(
                div().debug_selector(|| "zedx-option-push".into()).child(
                    Checkbox::new("zedx-option-push", push.into())
                        .label("Commit and Push…")
                        .label_size(ui::zedx::caption_size())
                        .disabled(busy)
                        .on_click(move |_, _, cx| {
                            push_panel.update(cx, |panel, cx| {
                                panel.zedx_commit.composer.push_by_default =
                                    !panel.zedx_commit.composer.push_by_default;
                                cx.notify();
                            })
                        }),
                ),
            )
    }
}

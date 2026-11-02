use super::*;

#[path = "zedx_commit_resize.rs"]
mod resize;

pub(super) fn enabled() -> bool {
    std::env::var_os("ZEDX_UPSTREAM_UI").is_none()
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommitTab {
    #[default]
    Commit,
    Stash,
    Shelf,
}

#[derive(Default)]
pub(super) struct CommitPanelState {
    tab: CommitTab,
    collapsed: [bool; 2],
    selected_stash: Option<git::Oid>,
    message: resize::MessageResize,
}

impl GitPanel {
    pub(super) fn confirm_panel_entry(
        &mut self,
        action: &menu::Confirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if enabled() && self.active_tab == GitPanelTab::Changes {
            if self.zedx_commit.tab == CommitTab::Commit {
                self.open_zedx_selected_file(window, cx);
            }
        } else {
            self.open_diff(action, window, cx);
        }
    }

    fn open_zedx_selected_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self
            .get_selected_entry()
            .and_then(GitListEntry::status_entry)
            .cloned()
        else {
            return;
        };
        self.open_zedx_file(entry, window, cx);
    }

    fn open_zedx_file(&self, entry: GitStatusEntry, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repository) = self.active_repository.clone() else {
            return;
        };
        // A click has one target even when the list has other marked or staged files.
        SoloDiffView::open_or_focus(entry, repository, self.workspace.clone(), window, cx)
            .detach_and_notify_err(self.workspace.clone(), window, cx);
    }

    pub(super) fn select_zedx_commit_tab(
        &mut self,
        tab: CommitTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.zedx_commit.tab = tab;
        self.set_active_tab(GitPanelTab::Changes, window, cx);
        cx.notify();
    }

    fn render_zedx_commit_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let counts = [
            self.change_entries_by_path()
                .filter(|entry| !entry.status.is_untracked())
                .count(),
            self.stash_entries.entries.len(),
            0,
        ];
        h_flex()
            .debug_selector(|| "zedx-commit-tabs".into())
            .h(px(35.0))
            .w_full()
            .flex_none()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .children(
                [
                    (CommitTab::Commit, "Commit"),
                    (CommitTab::Stash, "Stash"),
                    (CommitTab::Shelf, "Shelf"),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (tab, label))| {
                    let selected = self.zedx_commit.tab == tab;
                    h_flex()
                        .id(("zedx-commit-tab", index))
                        .debug_selector(move || format!("zedx-commit-tab-{label}"))
                        .relative()
                        .h_full()
                        .px(px(9.0))
                        .gap(px(6.0))
                        .cursor_pointer()
                        .hover(|style| style.bg(cx.theme().colors().ghost_element_hover))
                        .child(
                            Label::new(label)
                                .size(ui::zedx::label_size())
                                .color(if selected {
                                    Color::Default
                                } else {
                                    Color::Muted
                                }),
                        )
                        .child(
                            div()
                                .px(px(4.0))
                                .rounded(px(3.0))
                                .bg(cx.theme().colors().element_background)
                                .child(
                                    Label::new(counts[index].to_string())
                                        .size(ui::zedx::caption_size())
                                        .color(Color::Muted),
                                ),
                        )
                        .when(selected, |tab| {
                            tab.child(
                                div()
                                    .debug_selector(|| "zedx-commit-tab-active".into())
                                    .absolute()
                                    .left(px(6.0))
                                    .right(px(6.0))
                                    .bottom_0()
                                    .h(px(2.0))
                                    .rounded(px(2.0))
                                    .bg(cx.theme().colors().text_accent),
                            )
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_zedx_commit_tab(tab, window, cx)
                        }))
                }),
            )
            .child(div().flex_1())
            .child(
                ui::zedx::tool_button("zedx-commit-hide", ui::zedx::Icon::Minus, "Hide Commit", cx)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.close_panel(&Close, window, cx)),
                    ),
            )
    }

    fn render_zedx_commit_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .debug_selector(|| "zedx-commit-toolbar".into())
            .h(px(30.0))
            .w_full()
            .flex_none()
            .px(px(6.0))
            .gap(px(2.0))
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                ui::zedx::tool_button(
                    "zedx-commit-refresh",
                    ui::zedx::Icon::Refresh,
                    "Refresh",
                    cx,
                )
                .on_click(cx.listener(|this, _, window, cx| this.schedule_update(window, cx))),
            )
            .child(
                IconButton::new("zedx-commit-rollback", IconName::Undo)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Rollback selected files"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.revert_selected(&RestoreFile::default(), window, cx)
                    })),
            )
            .child(
                IconButton::new("zedx-commit-update", IconName::ArrowDown)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Update Project"))
                    .on_click(cx.listener(|this, _, window, cx| this.pull(false, window, cx))),
            )
            .child(self.render_view_options_menu("zedx-commit-view-options"))
            .child(
                ui::zedx::tool_button(
                    "zedx-commit-expand",
                    ui::zedx::Icon::Collapse,
                    "Expand All",
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.zedx_commit.collapsed = [false; 2];
                    cx.notify();
                })),
            )
            .child(
                IconButton::new("zedx-commit-collapse", IconName::ListCollapse)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Collapse All"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.zedx_commit.collapsed = [true; 2];
                        cx.notify();
                    })),
            )
    }

    fn render_zedx_change_group(
        &self,
        group: usize,
        label: &'static str,
        entries: Vec<GitStatusEntry>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let repository = self.active_repository.as_ref();
        let staged_count = entries
            .iter()
            .filter(|entry| {
                repository.is_some_and(|repo| {
                    Self::stage_status_for_entry(entry, repo.read(cx)) == StageStatus::Staged
                })
            })
            .count();
        let toggle = if staged_count == 0 {
            ToggleState::Unselected
        } else if staged_count == entries.len() {
            ToggleState::Selected
        } else {
            ToggleState::Indeterminate
        };
        let paths = entries
            .iter()
            .map(|entry| entry.repo_path.clone())
            .collect::<Vec<_>>();
        let can_write = self.has_write_access(cx);
        let collapsed = self.zedx_commit.collapsed[group];
        v_flex()
            .w_full()
            .min_w_0()
            .child(
                h_flex()
                    .h(px(26.0))
                    .gap(px(6.0))
                    .px(px(4.0))
                    .child(
                        IconButton::new(
                            ("zedx-commit-group", group),
                            if collapsed {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            },
                        )
                        .icon_size(IconSize::XSmall)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.zedx_commit.collapsed[group] = !collapsed;
                            cx.notify();
                        })),
                    )
                    .child(
                        Checkbox::new(("zedx-commit-group-check", group), toggle)
                            .disabled(!can_write || entries.is_empty())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let Some(repository) = this.active_repository.clone() else {
                                    return;
                                };
                                let task = repository.update(cx, |repository, cx| {
                                    if toggle == ToggleState::Selected {
                                        repository.unstage_entries(paths.clone(), cx)
                                    } else {
                                        repository.stage_entries(paths.clone(), cx)
                                    }
                                });
                                task.detach_and_notify_err(this.workspace.clone(), window, cx);
                            })),
                    )
                    .child(
                        Label::new(label)
                            .size(ui::zedx::label_size())
                            .weight(gpui::FontWeight::SEMIBOLD),
                    )
                    .child(
                        Label::new(format!("{} files", entries.len()))
                            .size(ui::zedx::caption_size())
                            .color(Color::Muted),
                    ),
            )
            .when(!collapsed, |group| {
                group.children(
                    entries
                        .into_iter()
                        .map(|entry| self.render_zedx_change_file(entry, cx).into_any_element()),
                )
            })
    }

    fn render_zedx_change_file(
        &self,
        entry: GitStatusEntry,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let path = entry.repo_path.as_unix_str().to_owned();
        let (directory, name) = path
            .rsplit_once('/')
            .map_or(("", path.as_str()), |(directory, name)| (directory, name));
        let name = name.to_owned();
        let directory = directory.to_owned();
        let index = self.entries.iter().position(|item| {
            item.status_entry()
                .is_some_and(|item| item.repo_path == entry.repo_path)
        });
        let selected = index.is_some() && self.selected_entry == index;
        let stage_status = self
            .active_repository
            .as_ref()
            .map_or(entry.staging, |repo| {
                Self::stage_status_for_entry(&entry, repo.read(cx))
            });
        let state = match stage_status {
            StageStatus::Staged => ToggleState::Selected,
            StageStatus::PartiallyStaged => ToggleState::Indeterminate,
            StageStatus::Unstaged => ToggleState::Unselected,
        };
        let checkbox_entry = entry.clone();
        let row_entry = entry.clone();
        h_flex()
            .id(SharedString::from(format!("zedx-commit-file-{path}")))
            .debug_selector(move || format!("zedx-commit-file-{path}"))
            .h(px(25.0))
            .w_full()
            .min_w_0()
            .pl(px(44.0))
            .pr(px(4.0))
            .gap(px(6.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .when(selected, |row| row.bg(cx.theme().colors().element_selected))
            .hover(|row| row.bg(cx.theme().colors().ghost_element_hover))
            .child(
                Checkbox::new(
                    SharedString::from(format!(
                        "zedx-commit-check-{}",
                        entry.repo_path.as_unix_str()
                    )),
                    state,
                )
                .disabled(!self.has_write_access(cx))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.toggle_staged_for_entry(
                        &GitListEntry::Status(checkbox_entry.clone()),
                        StageIntent::Toggle,
                        window,
                        cx,
                    );
                    cx.stop_propagation();
                })),
            )
            .child(
                Label::new(if entry.status.is_untracked() {
                    "?"
                } else if entry.status.is_deleted() {
                    "D"
                } else if entry.status.is_created() {
                    "A"
                } else {
                    "M"
                })
                .size(ui::zedx::caption_size())
                .color(Color::Accent),
            )
            .child(
                Label::new(name)
                    .size(ui::zedx::label_size())
                    .color(Color::Accent)
                    .truncate(),
            )
            .child(div().flex_1())
            .child(
                Label::new(directory)
                    .size(ui::zedx::caption_size())
                    .color(Color::Muted)
                    .truncate(),
            )
            .tooltip(Tooltip::text(entry.repo_path.as_unix_str().to_owned()))
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(index) = index {
                    this.clear_marks_and_select(index, cx);
                }
                this.open_zedx_file(row_entry.clone(), window, cx);
            }))
    }

    fn render_zedx_stashes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.zedx_commit.selected_stash;
        let selected_entry = self
            .stash_entries
            .entries
            .iter()
            .find(|entry| Some(entry.oid) == selected);
        let selected_index = selected_entry.map(|entry| entry.index);
        let can_apply = selected_index.is_some() && self.has_write_access(cx);
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h(px(30.0))
                    .flex_none()
                    .px(px(6.0))
                    .gap(px(2.0))
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        ui::zedx::tool_button(
                            "zedx-stash-refresh",
                            ui::zedx::Icon::Refresh,
                            "Refresh Stashes",
                            cx,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.schedule_update(window, cx)),
                        ),
                    )
                    .child(
                        IconButton::new("zedx-stash-apply", IconName::ArrowDown)
                            .icon_size(IconSize::Small)
                            .disabled(!can_apply)
                            .tooltip(Tooltip::text("Apply selected stash"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let Some(repository) = this.active_repository.clone() else {
                                    return;
                                };
                                let Some(index) = repository
                                    .read(cx)
                                    .cached_stash()
                                    .entries
                                    .iter()
                                    .find(|entry| Some(entry.oid) == selected)
                                    .map(|entry| entry.index)
                                else {
                                    return;
                                };
                                repository
                                    .update(cx, |repo, cx| repo.stash_apply(Some(index), cx))
                                    .detach_and_notify_err(this.workspace.clone(), window, cx);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .id("zedx-stash-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(4.0))
                    .when(self.stash_entries.entries.is_empty(), |list| {
                        list.child(
                            div().p_2().child(
                                Label::new("No stashes")
                                    .size(ui::zedx::label_size())
                                    .color(Color::Muted),
                            ),
                        )
                    })
                    .children(self.stash_entries.entries.iter().map(|entry| {
                        let oid = entry.oid;
                        h_flex()
                            .id(SharedString::from(format!("zedx-stash-{}", entry.index)))
                            .h(px(32.0))
                            .px(px(6.0))
                            .gap(px(6.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .when(Some(oid) == selected, |row| {
                                row.bg(cx.theme().colors().element_selected)
                            })
                            .hover(|row| row.bg(cx.theme().colors().ghost_element_hover))
                            .child(ui::zedx::icon(ui::zedx::Icon::Git, cx).size(px(16.0)))
                            .child(
                                Label::new(entry.message.clone())
                                    .size(ui::zedx::label_size())
                                    .truncate(),
                            )
                            .tooltip(Tooltip::text(entry.message.clone()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.zedx_commit.selected_stash = Some(oid);
                                cx.notify();
                            }))
                    })),
            )
    }

    pub(super) fn render_zedx_commit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.prepare_zedx_commit_message(cx);
        let message_height = self.zedx_commit_message_height(window);
        let mut changes = Vec::new();
        let mut unversioned = Vec::new();
        for entry in self.change_entries_by_path() {
            if entry.status.is_untracked() {
                unversioned.push(entry.clone());
            } else {
                changes.push(entry.clone());
            }
        }
        v_flex()
            .relative()
            .debug_selector(|| "zedx-commit-panel".into())
            .size_full()
            .min_h_0()
            .child(self.zedx_commit_resize_tracking(cx))
            .child(self.render_zedx_commit_tabs(cx))
            .map(|view| match self.zedx_commit.tab {
                CommitTab::Commit => view
                    .child(self.render_zedx_commit_toolbar(cx))
                    .child(
                        v_flex()
                            .id("zedx-commit-changes")
                            .flex_1()
                            .min_h(px(90.0))
                            .overflow_y_scroll()
                            .p(px(4.0))
                            .child(self.render_zedx_change_group(0, "Changes", changes, cx))
                            .child(self.render_zedx_change_group(
                                1,
                                "Unversioned Files",
                                unversioned,
                                cx,
                            )),
                    )
                    .when(self.active_repository.is_some(), |view| {
                        view.child(self.render_zedx_commit_resize(cx)).child(
                            v_flex()
                                .debug_selector(|| "zedx-commit-message".into())
                                .w_full()
                                .h(message_height)
                                .flex_none()
                                .min_h_0()
                                .overflow_hidden()
                                .children(self.render_footer(window, cx)),
                        )
                    }),
                CommitTab::Stash => view.child(self.render_zedx_stashes(cx)),
                CommitTab::Shelf => view.child(
                    v_flex()
                        .p_2()
                        .gap_2()
                        .child(
                            Label::new("Shelf is not available yet").size(ui::zedx::label_size()),
                        )
                        .child(
                            Label::new("Use Stash to save changes in this repository.")
                                .size(ui::zedx::caption_size())
                                .color(Color::Muted),
                        ),
                ),
            })
            .into_any_element()
    }
}

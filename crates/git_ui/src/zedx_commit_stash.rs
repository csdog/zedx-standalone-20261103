use super::*;

#[derive(Default)]
pub(super) struct State {
    repository: Option<gpui::EntityId>,
    pending: bool,
    error: Option<String>,
    scroll: gpui::ScrollHandle,
    metadata: std::collections::HashMap<git::Oid, Result<usize, String>>,
    metadata_loading: bool,
    metadata_task: Option<Task<()>>,
}

impl GitPanel {
    pub(super) fn load_zedx_stash_metadata(&mut self, cx: &mut Context<Self>) {
        if self.zedx_commit.tab != CommitTab::Stash || self.zedx_commit.stash.metadata_loading {
            return;
        }
        let Some(repository) = self.active_repository.clone() else {
            return;
        };
        let repository_id = repository.entity_id();
        let ids = self
            .stash_entries
            .entries
            .iter()
            .map(|entry| entry.oid)
            .filter(|oid| !self.zedx_commit.stash.metadata.contains_key(oid))
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return;
        }
        self.zedx_commit.stash.metadata_loading = true;
        self.zedx_commit.stash.metadata_task = Some(cx.spawn(async move |panel, cx| {
            for oid in ids {
                let request =
                    repository.update(cx, |repository, _| repository.zedx_stash_file_count(oid));
                let result = request
                    .await
                    .unwrap_or_else(|error| Err(error.into()))
                    .map_err(|error| error.to_string());
                let current = panel.update(cx, |panel, cx| {
                    if panel.active_repository.as_ref().map(Entity::entity_id)
                        != Some(repository_id)
                    {
                        return false;
                    }
                    panel.zedx_commit.stash.metadata.insert(oid, result);
                    cx.notify();
                    true
                });
                match current {
                    Ok(true) => {}
                    Ok(false) => return,
                    Err(error) => {
                        log::error!("Loading stash metadata: {error}");
                        return;
                    }
                }
            }
            panel
                .update(cx, |panel, cx| {
                    if panel.active_repository.as_ref().map(Entity::entity_id)
                        == Some(repository_id)
                    {
                        panel.zedx_commit.stash.metadata_loading = false;
                        cx.notify();
                    }
                })
                .log_err();
        }));
    }

    pub(super) fn synchronize_zedx_stash(&mut self) {
        let repository = self.active_repository.as_ref().map(Entity::entity_id);
        if repository != self.zedx_commit.stash.repository {
            self.zedx_commit.stash.repository = repository;
            self.zedx_commit.stash.metadata.clear();
            self.zedx_commit.stash.metadata_loading = false;
            self.zedx_commit.stash.metadata_task = None;
            self.zedx_commit.selected_stash = None;
            self.zedx_commit.stash.error = None;
        }
        if self.zedx_commit.selected_stash.is_some_and(|oid| {
            !self
                .stash_entries
                .entries
                .iter()
                .any(|entry| entry.oid == oid)
        }) {
            self.zedx_commit.selected_stash = None;
        }
    }

    pub(crate) fn select_zedx_stash(&mut self, previous: bool, cx: &mut Context<Self>) -> bool {
        if !enabled()
            || self.active_tab != GitPanelTab::Changes
            || self.zedx_commit.tab != CommitTab::Stash
        {
            return false;
        }
        self.synchronize_zedx_stash();
        let entries = &self.stash_entries.entries;
        if entries.is_empty() {
            return true;
        }
        let selected = entries
            .iter()
            .position(|entry| Some(entry.oid) == self.zedx_commit.selected_stash);
        let index = match selected {
            None if previous => entries.len() - 1,
            None => 0,
            Some(0) if previous => entries.len() - 1,
            Some(index) if previous => index - 1,
            Some(index) => (index + 1) % entries.len(),
        };
        self.zedx_commit.selected_stash = entries.get(index).map(|entry| entry.oid);
        self.zedx_commit.stash.scroll.scroll_to_item(index);
        cx.notify();
        true
    }

    pub(super) fn open_zedx_selected_stash(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repository) = self.active_repository.as_ref() else {
            return;
        };
        if Some(repository.entity_id()) != self.zedx_commit.stash.repository {
            return;
        }
        let Some(entry) = repository
            .read(cx)
            .cached_stash()
            .entries
            .iter()
            .find(|entry| Some(entry.oid) == self.zedx_commit.selected_stash)
            .cloned()
        else {
            return;
        };
        CommitView::open(
            entry.oid.to_string(),
            repository.downgrade(),
            self.workspace.clone(),
            Some(entry.index),
            None,
            window,
            cx,
        );
    }

    fn operate_zedx_stash(
        &mut self,
        drop_entry: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.zedx_commit.stash.pending || !self.has_write_access(cx) {
            return;
        }
        let Some(repository) = self.active_repository.clone() else {
            return;
        };
        if Some(repository.entity_id()) != self.zedx_commit.stash.repository {
            return;
        }
        let Some(entry) = repository
            .read(cx)
            .cached_stash()
            .entries
            .iter()
            .find(|entry| Some(entry.oid) == self.zedx_commit.selected_stash)
            .cloned()
        else {
            return;
        };
        let confirmation = drop_entry.then(|| {
            window.prompt(
                PromptLevel::Warning,
                "Drop this stash?",
                Some(&entry.message),
                &["Drop Stash", "Cancel"],
                cx,
            )
        });
        self.zedx_commit.stash.pending = true;
        self.zedx_commit.stash.error = None;
        cx.notify();
        cx.spawn_in(window, async move |panel, cx| {
            let result: anyhow::Result<bool> = async {
                if let Some(confirmation) = confirmation && confirmation.await? != 0 { return Ok(false); }
                let task = panel.update_in(cx, |panel, _, cx| {
                    if panel.active_repository.as_ref().map(Entity::entity_id) != Some(repository.entity_id()) || !panel.has_write_access(cx) {
                        anyhow::bail!("The active repository changed. Select a stash in the current repository.");
                    }
                    // Indices move when another stash is added or removed while the prompt is open.
                    let index = repository.read(cx).cached_stash().entries.iter()
                        .find(|candidate| candidate.oid == entry.oid)
                        .map(|entry| entry.index)
                        .context("The selected stash no longer exists")?;
                    Ok(repository.update(cx, |repository, cx| {
                        if drop_entry {
                            let result = repository.stash_drop(Some(index), cx);
                            cx.spawn(async move |_, _| result.await?)
                        } else {
                            repository.stash_apply(Some(index), cx)
                        }
                    }))
                })??;
                task.await?;
                Ok(true)
            }.await;
            panel.update_in(cx, |panel, window, cx| {
                panel.zedx_commit.stash.pending = false;
                match result {
                    Ok(true) => {
                        if drop_entry
                            && panel.active_repository.as_ref().map(Entity::entity_id) == Some(repository.entity_id())
                            && panel.zedx_commit.selected_stash == Some(entry.oid) {
                            panel.zedx_commit.selected_stash = None;
                        }
                    }
                    Ok(false) => {}
                    Err(error) => panel.zedx_commit.stash.error = Some(error.to_string()),
                }
                panel.schedule_update(window, cx);
                cx.notify();
            }).log_err();
        }).detach();
    }

    pub(super) fn render_zedx_stashes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.zedx_commit.selected_stash;
        let available = !self.zedx_commit.stash.pending
            && self.has_write_access(cx)
            && self
                .stash_entries
                .entries
                .iter()
                .any(|entry| Some(entry.oid) == selected);
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .debug_selector(|| "zedx-stash-toolbar".into())
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
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.zedx_commit.stash.metadata.clear();
                            this.zedx_commit.stash.metadata_loading = false;
                            this.zedx_commit.stash.metadata_task = None;
                            this.schedule_update(window, cx);
                        })),
                    )
                    .child(
                        div().debug_selector(|| "zedx-stash-apply".into()).child(
                            IconButton::new("zedx-stash-apply", IconName::ArrowDown)
                                .icon_size(IconSize::Small)
                                .disabled(!available)
                                .tooltip(Tooltip::text("Apply selected stash"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.operate_zedx_stash(false, window, cx)
                                })),
                        ),
                    )
                    .child(
                        div().debug_selector(|| "zedx-stash-drop".into()).child(
                            IconButton::new("zedx-stash-drop", IconName::Trash)
                                .icon_size(IconSize::Small)
                                .disabled(!available)
                                .tooltip(Tooltip::text("Drop selected stash"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.operate_zedx_stash(true, window, cx)
                                })),
                        ),
                    ),
            )
            .when_some(self.zedx_commit.stash.error.clone(), |view, error| {
                view.child(
                    div()
                        .debug_selector(|| "zedx-stash-error".into())
                        .p_2()
                        .child(
                            Label::new(error)
                                .size(ui::zedx::caption_size())
                                .color(Color::Error),
                        ),
                )
            })
            .child(
                v_flex()
                    .id("zedx-stash-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.zedx_commit.stash.scroll)
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
                            .debug_selector(move || format!("zedx-stash-row-{oid}"))
                            .h(px(32.0))
                            .flex_none()
                            .min_w_0()
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
                                div().flex_1().min_w_0().child(
                                    Label::new(entry.message.clone())
                                        .size(ui::zedx::label_size())
                                        .truncate(),
                                ),
                            )
                            .child(
                                Label::new(format!(
                                    "{} · {}",
                                    match self.zedx_commit.stash.metadata.get(&oid) {
                                        Some(Ok(count)) => format!("{count} files"),
                                        Some(Err(_)) => "? files".into(),
                                        None => "… files".into(),
                                    },
                                    relative_age(entry.timestamp)
                                ))
                                .size(ui::zedx::caption_size())
                                .color(Color::Muted),
                            )
                            .tooltip(Tooltip::text(format!(
                                "{}\nstash@{{{}}}\n{}\nEnter or double-click to view changes",
                                entry.message,
                                entry.index,
                                self.zedx_commit
                                    .stash
                                    .metadata
                                    .get(&oid)
                                    .and_then(|result| result.as_ref().err())
                                    .map(String::as_str)
                                    .unwrap_or("")
                            )))
                            .on_click(cx.listener(
                                move |this, event: &gpui::ClickEvent, window, cx| {
                                    window.focus(&this.focus_handle, cx);
                                    this.zedx_commit.selected_stash = Some(oid);
                                    this.zedx_commit.stash.error = None;
                                    if event.click_count() == 2 {
                                        this.open_zedx_selected_stash(window, cx);
                                    }
                                    cx.notify();
                                },
                            ))
                    })),
            )
    }
}

pub(super) fn relative_age(timestamp: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    age_at(now, timestamp)
}
fn age_at(now: u64, timestamp: i64) -> String {
    let seconds = now.saturating_sub(timestamp.max(0) as u64);
    if seconds < 60 {
        "just now".into()
    } else if seconds < 3600 {
        format!("{} min ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{} h ago", seconds / 3600)
    } else {
        format!("{} d ago", seconds / 86400)
    }
}
#[cfg(test)]
mod metadata_tests {
    use super::*;
    #[test]
    fn relative_age_handles_future_and_time_boundaries() {
        assert_eq!(age_at(10, 20), "just now");
        assert_eq!(age_at(60, 0), "1 min ago");
        assert_eq!(age_at(3600, 0), "1 h ago");
        assert_eq!(age_at(86400, 0), "1 d ago");
        assert_eq!(age_at(0, -10), "just now");
    }
}

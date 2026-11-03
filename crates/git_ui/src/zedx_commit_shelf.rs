use super::*;
use git::repository::ShelfEntry;
gpui::actions!(zedx_shelf, [DeleteShelf]);
pub(super) fn bind_keys(cx: &mut App) {
    if !enabled() {
        return;
    }
    for key in ["delete", "backspace"] {
        cx.bind_keys([gpui::KeyBinding::new(
            key,
            DeleteShelf,
            Some("GitPanel && ZedxShelf && ChangesList"),
        )]);
    }
}

#[derive(Default)]
pub(super) struct State {
    repository: Option<gpui::EntityId>,
    pub(super) entries: Vec<ShelfEntry>,
    selected: Option<ShelfEntry>,
    loaded: bool,
    pending: bool,
    error: Option<String>,
    generation: u64,
    scroll: gpui::ScrollHandle,
}

impl GitPanel {
    pub(crate) fn is_zedx_shelf(&self) -> bool {
        enabled()
            && self.active_tab == GitPanelTab::Changes
            && self.zedx_commit.tab == CommitTab::Shelf
    }
    pub(crate) fn select_zedx_shelf(&mut self, previous: bool, cx: &mut Context<Self>) -> bool {
        if !self.is_zedx_shelf() {
            return false;
        }
        let state = &mut self.zedx_commit.shelf;
        if state.pending
            || state.repository != self.active_repository.as_ref().map(Entity::entity_id)
        {
            return true;
        }
        let count = state.entries.len();
        if count == 0 {
            return true;
        }
        let current = state.selected.as_ref().and_then(|selected| {
            state
                .entries
                .iter()
                .position(|entry| entry.id == selected.id)
        });
        let index = match (current, previous) {
            (None, true) | (Some(0), true) => count - 1,
            (None, false) => 0,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % count,
        };
        state.selected = state.entries.get(index).cloned();
        state.scroll.scroll_to_item(index);
        cx.notify();
        true
    }
    pub(crate) fn select_zedx_shelf_edge(&mut self, last: bool, cx: &mut Context<Self>) -> bool {
        if !self.is_zedx_shelf() {
            return false;
        }
        let state = &mut self.zedx_commit.shelf;
        if state.pending
            || state.repository != self.active_repository.as_ref().map(Entity::entity_id)
        {
            return true;
        }
        let index = if last {
            state.entries.len().saturating_sub(1)
        } else {
            0
        };
        state.selected = state.entries.get(index).cloned();
        state.scroll.scroll_to_item(index);
        cx.notify();
        true
    }
    pub(super) fn apply_zedx_selected_shelf(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_zedx_shelf() {
            self.operate_zedx_shelf(Operation::Apply, window, cx);
        }
    }
    pub(crate) fn delete_zedx_selected_shelf(
        &mut self,
        _: &DeleteShelf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_zedx_shelf() {
            self.operate_zedx_shelf(Operation::Delete, window, cx);
        }
    }
    #[cfg(test)]
    pub(crate) fn set_zedx_shelves_for_test(
        &mut self,
        entries: Vec<ShelfEntry>,
        cx: &mut Context<Self>,
    ) {
        let generation = self.zedx_commit.shelf.generation + 1;
        self.zedx_commit.shelf = State {
            entries,
            repository: self.active_repository.as_ref().map(Entity::entity_id),
            loaded: true,
            generation,
            ..Default::default()
        };
        cx.notify();
    }
    #[cfg(test)]
    pub(crate) fn zedx_selected_shelf_for_test(&self) -> Option<ShelfEntry> {
        self.zedx_commit.shelf.selected.clone()
    }

    pub(super) fn synchronize_zedx_shelf(&mut self, cx: &mut Context<Self>) {
        let repository = self.active_repository.as_ref().map(Entity::entity_id);
        if self.zedx_commit.shelf.repository != repository {
            self.zedx_commit.shelf = State {
                repository,
                generation: self.zedx_commit.shelf.generation + 1,
                ..Default::default()
            };
        }
        if self.zedx_commit.tab == CommitTab::Shelf
            && !self.zedx_commit.shelf.loaded
            && !self.zedx_commit.shelf.pending
        {
            self.refresh_zedx_shelves(cx);
        }
    }
    fn refresh_zedx_shelves(&mut self, cx: &mut Context<Self>) {
        if self.zedx_commit.shelf.pending {
            return;
        }
        let Some(repository) = self.active_repository.clone() else {
            return;
        };
        let repository_id = repository.entity_id();
        self.zedx_commit.shelf.generation += 1;
        let generation = self.zedx_commit.shelf.generation;
        self.zedx_commit.shelf.pending = true;
        let request = repository.update(cx, |repository, _| repository.zedx_shelves());
        cx.spawn(async move |panel, cx| {
            let result = request.await.unwrap_or_else(|error| Err(error.into()));
            panel
                .update(cx, |panel, cx| {
                    let state = &mut panel.zedx_commit.shelf;
                    if panel.active_repository.as_ref().map(Entity::entity_id)
                        != Some(repository_id)
                        || state.generation != generation
                    {
                        return;
                    }
                    state.pending = false;
                    state.loaded = true;
                    match result {
                        Ok(entries) => {
                            if state.selected.as_ref().is_some_and(|selected| {
                                !entries.iter().any(|entry| entry.id == selected.id)
                            }) {
                                state.selected = None;
                            }
                            state.entries = entries;
                            state.error = None;
                        }
                        Err(error) => state.error = Some(error.to_string()),
                    }
                    cx.notify();
                })
                .log_err();
        })
        .detach();
    }
    fn operate_zedx_shelf(
        &mut self,
        operation: Operation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.zedx_commit.shelf.pending || !self.has_write_access(cx) {
            return;
        }
        let Some(repository) = self.active_repository.clone() else {
            return;
        };
        let repository_id = repository.entity_id();
        if self.zedx_commit.shelf.repository != Some(repository_id) {
            self.synchronize_zedx_shelf(cx);
            return;
        }
        let generation = self.zedx_commit.shelf.generation;
        let selected = self.zedx_commit.shelf.selected.clone();
        if !matches!(operation, Operation::Save) && selected.is_none() {
            return;
        }
        if !matches!(operation, Operation::Delete)
            && self
                .workspace
                .upgrade()
                .is_some_and(|workspace| workspace.read(cx).items(cx).any(|item| item.is_dirty(cx)))
        {
            self.zedx_commit.shelf.error =
                Some("Save open files before shelving or applying a shelf.".into());
            cx.notify();
            return;
        }
        let message = self.commit_editor.read(cx).text(cx);
        let confirmation = window.prompt(PromptLevel::Warning, match operation {
            Operation::Save => "Shelve all saved changes?", Operation::Apply => "Apply selected shelf?", Operation::Delete => "Delete selected shelf?",
        }, Some(match operation {
            Operation::Save => "Tracked and untracked changes will be saved to this repository's Shelf and removed from the working tree. Ignored files remain untouched.",
            Operation::Apply => "Changes and their staged state will be restored. The shelf will remain available.",
            Operation::Delete => "This shelf will be permanently deleted. Working files remain untouched.",
        }), &["Continue", "Cancel"], cx);
        self.zedx_commit.shelf.pending = true;
        self.zedx_commit.shelf.error = None;
        cx.notify();
        cx.spawn_in(window, async move |panel, cx| {
            let result: anyhow::Result<bool> = async {
                if confirmation.await? != 0 {
                    return Ok(false);
                }
                let request = panel.update(cx, |panel, cx| {
                    anyhow::ensure!(
                        panel.active_repository.as_ref().map(Entity::entity_id)
                            == Some(repository_id)
                            && panel.has_write_access(cx),
                        "The active repository changed or became read only."
                    );
                    if !matches!(operation, Operation::Delete) {
                        anyhow::ensure!(
                            !panel.workspace.upgrade().is_some_and(|workspace| workspace
                                .read(cx)
                                .items(cx)
                                .any(|item| item.is_dirty(cx))),
                            "Save open files before applying this operation."
                        );
                    }
                    repository.update(cx, |repository, cx| {
                        let task = match operation {
                            Operation::Save => {
                                let request = repository.zedx_shelve(message);
                                cx.spawn(async move |_, _| {
                                    request.await??;
                                    Ok(())
                                })
                            }
                            Operation::Apply | Operation::Delete => {
                                let id = selected.as_ref().context("No shelf is selected")?.id;
                                let request = if matches!(operation, Operation::Apply) {
                                    repository.zedx_apply_shelf(id)
                                } else {
                                    repository.zedx_delete_shelf(id)
                                };
                                cx.spawn(async move |_, _| request.await?)
                            }
                        };
                        Ok::<_, anyhow::Error>(task)
                    })
                })??;
                request.await?;
                Ok(true)
            }
            .await;
            panel
                .update_in(cx, |panel, window, cx| {
                    if panel.active_repository.as_ref().map(Entity::entity_id)
                        != Some(repository_id)
                        || panel.zedx_commit.shelf.generation != generation
                    {
                        return;
                    }
                    panel.zedx_commit.shelf.pending = false;
                    match result {
                        Ok(true) => {
                            panel.zedx_commit.shelf.loaded = false;
                            panel.schedule_update(window, cx);
                        }
                        Ok(false) => {}
                        Err(error) => panel.zedx_commit.shelf.error = Some(error.to_string()),
                    }
                    cx.notify();
                })
                .log_err();
        })
        .detach();
    }
    pub(super) fn render_zedx_shelves(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = &self.zedx_commit.shelf;
        let writable =
            !state.pending && self.has_write_access(cx) && self.active_repository.is_some();
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h(px(32.0))
                    .px_1()
                    .gap_1()
                    .flex_none()
                    .child(
                        Button::new("shelf-save", "Shelve All…")
                            .disabled(!writable)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.operate_zedx_shelf(Operation::Save, window, cx)
                            })),
                    )
                    .child(
                        IconButton::new("shelf-apply", IconName::ArrowDown)
                            .tooltip(Tooltip::text("Apply selected shelf…"))
                            .disabled(!writable || state.selected.is_none())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.operate_zedx_shelf(Operation::Apply, window, cx)
                            })),
                    )
                    .child(
                        IconButton::new("shelf-delete", IconName::Trash)
                            .tooltip(Tooltip::text("Delete selected shelf…"))
                            .disabled(!writable || state.selected.is_none())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.operate_zedx_shelf(Operation::Delete, window, cx)
                            })),
                    )
                    .child(
                        IconButton::new("shelf-refresh", IconName::RotateCw)
                            .tooltip(Tooltip::text("Refresh Shelves"))
                            .disabled(state.pending)
                            .on_click(cx.listener(|this, _, _, cx| this.refresh_zedx_shelves(cx))),
                    ),
            )
            .when(state.pending, |view| {
                view.child(
                    div()
                        .p_2()
                        .child(Label::new("Working…").color(Color::Muted)),
                )
            })
            .when_some(state.error.clone(), |view, error| {
                view.child(
                    div()
                        .debug_selector(|| "zedx-shelf-error".into())
                        .p_2()
                        .child(Label::new(error).color(Color::Error)),
                )
            })
            .child(
                v_flex()
                    .id("zedx-shelf-list")
                    .debug_selector(|| "zedx-shelf-list".into())
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&state.scroll)
                    .when(
                        state.loaded && state.entries.is_empty() && state.error.is_none(),
                        |view| {
                            view.child(
                                div()
                                    .p_3()
                                    .child(Label::new("No shelves").color(Color::Muted)),
                            )
                        },
                    )
                    .children(state.entries.iter().map(|entry| {
                        let selected = state
                            .selected
                            .as_ref()
                            .is_some_and(|selected| selected.id == entry.id);
                        let entry = entry.clone();
                        ui::ButtonLike::new(SharedString::from(format!("shelf-{}", entry.id)))
                            .full_width()
                            .toggle_state(selected)
                            .child(
                                v_flex()
                                    .p_2()
                                    .min_w_0()
                                    .child(
                                        Label::new(entry.message.clone())
                                            .size(ui::zedx::label_size())
                                            .truncate(),
                                    )
                                    .child(
                                        Label::new(format!(
                                            "{} files · {}",
                                            entry.file_count,
                                            super::stash::relative_age(entry.timestamp as i64)
                                        ))
                                        .size(ui::zedx::caption_size())
                                        .color(Color::Muted),
                                    ),
                            )
                            .tooltip(Tooltip::text("Enter to apply · Delete to remove"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                window.focus(&this.focus_handle, cx);
                                this.zedx_commit.shelf.selected = Some(entry.clone());
                                cx.notify();
                            }))
                    })),
            )
    }
}
#[derive(Clone, Copy)]
enum Operation {
    Save,
    Apply,
    Delete,
}

use super::*;
#[path = "zedx_library_discovery.rs"]
mod discovery;
#[path = "zedx_scratch_menu.rs"]
mod scratch_menu;

pub(super) fn enabled() -> bool {
    std::env::var_os("ZEDX_UPSTREAM_UI").is_none()
}

#[derive(Default)]
pub(super) struct Groups {
    expanded: HashSet<String>,
    libraries: discovery::Discovery,
    selected: Option<String>,
    rows: Vec<Row>,
    scratch: Option<(Entity<Worktree>, Arc<RelPath>)>,
    scratch_loading: bool,
    scratch_error: Option<String>,
}

#[derive(Clone)]
enum Target {
    Libraries,
    RefreshLibraries,
    ScratchGroup,
    Scratches,
    Folder(ProjectPath, ProjectEntryId),
    File(ProjectPath),
    Console,
    Message,
}

#[derive(Clone)]
pub(super) struct Row {
    key: String,
    label: String,
    depth: usize,
    target: Target,
    glyph: ui::zedx::Icon,
}

impl Row {
    fn group(key: &str, label: &str, depth: usize, target: Target, glyph: ui::zedx::Icon) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            depth,
            target,
            glyph,
        }
    }

    fn expandable(&self) -> bool {
        matches!(
            self.target,
            Target::Libraries | Target::ScratchGroup | Target::Scratches | Target::Folder(..)
        )
    }
}

fn append_children(
    rows: &mut Vec<Row>,
    snapshot: &worktree::Snapshot,
    worktree_id: WorktreeId,
    parent: &RelPath,
    depth: usize,
    expanded: &HashSet<String>,
) {
    let mut children = snapshot.child_entries(parent).collect::<Vec<_>>();
    children.sort_by(|left, right| {
        right
            .is_dir()
            .cmp(&left.is_dir())
            .then_with(|| {
                left.path
                    .as_unix_str()
                    .to_lowercase()
                    .cmp(&right.path.as_unix_str().to_lowercase())
            })
            .then_with(|| left.path.cmp(&right.path))
    });
    for entry in children {
        let path = ProjectPath {
            worktree_id,
            path: entry.path.clone(),
        };
        let key = format!("source-{worktree_id:?}-{}", entry.path.as_unix_str());
        let is_expanded = expanded.contains(&key);
        rows.push(Row {
            key,
            label: entry.path.file_name().unwrap_or_default().to_owned(),
            depth,
            target: if entry.is_dir() {
                Target::Folder(path, entry.id)
            } else {
                Target::File(path)
            },
            glyph: if entry.is_dir() {
                ui::zedx::Icon::Folder
            } else {
                ui::zedx::Icon::Code
            },
        });
        if entry.is_dir() && is_expanded {
            append_children(
                rows,
                snapshot,
                worktree_id,
                &entry.path,
                depth + 1,
                expanded,
            );
        }
    }
}

impl ProjectPanel {
    pub(super) fn refresh_zedx_groups(&mut self, cx: &App) {
        if !enabled() {
            self.zedx_groups.rows.clear();
            self.zedx_groups.selected = None;
            return;
        }
        if self.selection.is_some() && !self.has_zedx_scratch_selection(cx) {
            self.zedx_groups.selected = None;
        }
        let roots = self
            .project
            .read(cx)
            .visible_worktrees(cx)
            .map(|tree| tree.read(cx).abs_path().to_path_buf())
            .collect::<Vec<_>>();
        if !self.zedx_groups.libraries.roots.is_empty() && roots != self.zedx_groups.libraries.roots
        {
            self.zedx_groups.libraries.reset(roots);
        }
        let state = &self.zedx_groups;
        let mut rows = vec![Row::group(
            "external-libraries",
            "External Libraries",
            0,
            Target::Libraries,
            ui::zedx::Icon::Library,
        )];
        if state.expanded.contains("external-libraries") {
            for source in &state.libraries.sources {
                let worktree = source.worktree.read(cx);
                let snapshot = worktree.snapshot();
                let Some(entry) = snapshot.entry_for_path(&source.prefix) else {
                    continue;
                };
                let key = format!("discovered-library-{}", source.absolute.display());
                rows.push(Row {
                    key: key.clone(),
                    label: source.label.clone(),
                    depth: 1,
                    target: Target::Folder(
                        ProjectPath {
                            worktree_id: worktree.id(),
                            path: source.prefix.clone(),
                        },
                        entry.id,
                    ),
                    glyph: ui::zedx::Icon::Library,
                });
                if state.expanded.contains(&key) {
                    append_children(
                        &mut rows,
                        &snapshot,
                        worktree.id(),
                        &source.prefix,
                        2,
                        &state.expanded,
                    );
                }
            }
            let project = self.project.read(cx);
            let visible = project
                .visible_worktrees(cx)
                .map(|worktree| worktree.read(cx).abs_path())
                .collect::<Vec<_>>();
            let mut libraries = project
                .worktrees(cx)
                .filter(|worktree| {
                    let worktree = worktree.read(cx);
                    !worktree.is_visible()
                        && !state
                            .libraries
                            .sources
                            .iter()
                            .any(|source| source.absolute == worktree.abs_path().as_ref())
                        && !visible
                            .iter()
                            .any(|root| worktree.abs_path().starts_with(root.as_ref()))
                        && !worktree
                            .abs_path()
                            .starts_with(paths::data_dir().join("scratches"))
                })
                .collect::<Vec<_>>();
            libraries.sort_by_key(|worktree| worktree.read(cx).abs_path());
            for worktree in libraries {
                let worktree = worktree.read(cx);
                let snapshot = worktree.snapshot();
                let Some(root) = snapshot.root_entry() else {
                    continue;
                };
                let key = format!("library-{:?}", worktree.id());
                let path = ProjectPath {
                    worktree_id: worktree.id(),
                    path: root.path.clone(),
                };
                rows.push(Row {
                    key: key.clone(),
                    label: worktree.abs_path().to_string_lossy().into_owned(),
                    depth: 1,
                    target: if root.is_dir() {
                        Target::Folder(path, root.id)
                    } else {
                        Target::File(path)
                    },
                    glyph: if root.is_dir() {
                        ui::zedx::Icon::Folder
                    } else {
                        ui::zedx::Icon::Code
                    },
                });
                if root.is_dir() && state.expanded.contains(&key) {
                    append_children(
                        &mut rows,
                        &snapshot,
                        worktree.id(),
                        &root.path,
                        2,
                        &state.expanded,
                    );
                }
            }
            if rows.len() == 1 {
                rows.push(Row::group(
                    "libraries-empty",
                    "No external sources loaded",
                    1,
                    Target::Message,
                    ui::zedx::Icon::Library,
                ));
            }
            rows.push(Row::group(
                "libraries-refresh",
                if state.libraries.pending {
                    "Discovering libraries…"
                } else {
                    "Refresh Libraries"
                },
                1,
                Target::RefreshLibraries,
                ui::zedx::Icon::Refresh,
            ));
            if let Some(error) = &state.libraries.error {
                rows.push(Row::group(
                    "libraries-error",
                    error,
                    1,
                    Target::Message,
                    ui::zedx::Icon::Warning,
                ));
            }
        }
        rows.push(Row::group(
            "scratches-and-consoles",
            "Scratches and Consoles",
            0,
            Target::ScratchGroup,
            ui::zedx::Icon::Scratches,
        ));
        if state.expanded.contains("scratches-and-consoles") {
            rows.push(Row::group(
                "scratches",
                "Scratches",
                1,
                Target::Scratches,
                ui::zedx::Icon::Scratches,
            ));
            if state.expanded.contains("scratches") {
                if self.project.read(cx).is_local() {
                    if let Some((worktree, prefix)) = state.scratch.as_ref() {
                        let worktree = worktree.read(cx);
                        append_children(
                            &mut rows,
                            &worktree.snapshot(),
                            worktree.id(),
                            prefix,
                            2,
                            &state.expanded,
                        );
                    }
                } else {
                    rows.push(Row::group(
                        "scratch-local-only",
                        "Available in local projects",
                        2,
                        Target::Message,
                        ui::zedx::Icon::Scratches,
                    ));
                }
                if let Some(error) = &state.scratch_error {
                    rows.push(Row::group(
                        "scratch-error",
                        error,
                        2,
                        Target::Message,
                        ui::zedx::Icon::Warning,
                    ));
                }
            }
            rows.push(Row::group(
                "consoles",
                "Consoles",
                1,
                Target::Console,
                ui::zedx::Icon::Terminal,
            ));
        }
        if state
            .selected
            .as_ref()
            .is_some_and(|key| !rows.iter().any(|row| row.key == *key))
        {
            self.zedx_groups.selected = None;
        }
        if self.has_zedx_scratch_selection(cx) {
            let selected = self.selection;
            let current = self.zedx_groups.selected.as_ref().and_then(|key| {
                rows.iter().find(|row| {
                    &row.key == key && self.zedx_scratch_entry(&row.target, cx) == selected
                })
            });
            self.zedx_groups.selected = current
                .or_else(|| {
                    rows.iter()
                        .find(|row| self.zedx_scratch_entry(&row.target, cx) == selected)
                })
                .map(|row| row.key.clone());
        }
        self.zedx_groups.rows = rows;
    }

    pub(super) fn limit_zedx_sticky_entries(&self, entries: &mut SmallVec<[AnyElement; 8]>) {
        if !enabled() {
            return;
        }
        let Some(height) = self.entry_row_height() else {
            return;
        };
        // Small split panels must retain room for a source row below their pinned ancestors.
        let capacity = (f32::from(self.scroll_handle.viewport().size.height) / f32::from(height))
            .floor()
            .max(0.0) as usize;
        let keep = capacity.saturating_sub(1);
        if entries.len() > keep {
            entries.drain(..entries.len() - keep).for_each(drop);
        }
    }

    pub(super) fn zedx_group_count(&self) -> usize {
        self.zedx_groups.rows.len()
    }

    fn select_zedx_group(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(row) = self.zedx_groups.rows.get(index) {
            self.zedx_groups.selected = Some(row.key.clone());
            self.selection = match row.target {
                Target::File(_) | Target::Folder(_, _) => self.zedx_scratch_entry(&row.target, cx),
                _ => None,
            };
            self.marked_entries.clear();
            let regular: usize = self
                .state
                .visible_entries
                .iter()
                .map(|tree| tree.entries.len())
                .sum();
            self.scroll_handle
                .scroll_to_item(regular + index, ScrollStrategy::Nearest);
            self.focus_handle.focus(window, cx);
            cx.notify();
        }
    }

    pub(super) fn select_zedx_group_step(
        &mut self,
        previous: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !enabled() || self.state.edit_state.is_some() {
            return false;
        }
        if self.selection.is_some() && !self.has_zedx_scratch_selection(cx) {
            self.zedx_groups.selected = None;
        }
        if let Some(index) = self
            .zedx_groups
            .rows
            .iter()
            .position(|row| Some(&row.key) == self.zedx_groups.selected.as_ref())
        {
            if previous && index == 0 {
                self.zedx_groups.selected = None;
                self.select_last(&SelectLast, window, cx);
            } else {
                let target = if previous {
                    index - 1
                } else {
                    (index + 1).min(self.zedx_groups.rows.len().saturating_sub(1))
                };
                self.select_zedx_group(target, window, cx);
            }
            return true;
        }
        if !previous
            && !window.modifiers().shift
            && self.selection.is_some_and(|selection| {
                self.state
                    .visible_entries
                    .last()
                    .and_then(|tree| tree.entries.last())
                    .is_some_and(|entry| entry.id == selection.entry_id)
            })
            && !self.zedx_groups.rows.is_empty()
        {
            self.select_zedx_group(0, window, cx);
            return true;
        }
        false
    }

    pub(super) fn activate_zedx_group(
        &mut self,
        expand: Option<bool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !enabled()
            || self.state.edit_state.is_some()
            || (self.selection.is_some() && !self.has_zedx_scratch_selection(cx))
        {
            return false;
        }
        let Some(row) = self
            .zedx_groups
            .rows
            .iter()
            .find(|row| Some(&row.key) == self.zedx_groups.selected.as_ref())
            .cloned()
        else {
            return false;
        };
        if row.expandable() {
            let opening = expand.unwrap_or(!self.zedx_groups.expanded.contains(&row.key));
            if opening {
                self.zedx_groups.expanded.insert(row.key);
                match row.target {
                    Target::Libraries => self.load_zedx_libraries(cx),
                    Target::Scratches => self.load_zedx_scratches(false, window, cx),
                    Target::Folder(path, entry_id) => {
                        let task = self.project.update(cx, |project, cx| {
                            project.expand_entry(path.worktree_id, entry_id, cx)
                        });
                        if let Some(task) = task {
                            task.detach_and_notify_err(self.workspace.clone(), window, cx);
                        }
                    }
                    _ => {}
                }
            } else {
                self.zedx_groups.expanded.remove(&row.key);
            }
        } else if expand != Some(false) {
            match row.target {
                Target::File(path) => {
                    if let Some(workspace) = self.workspace.upgrade() {
                        workspace
                            .update(cx, |workspace, cx| {
                                workspace.open_path(path, None, true, window, cx)
                            })
                            .detach_and_notify_err(self.workspace.clone(), window, cx);
                    }
                }
                Target::RefreshLibraries => self.load_zedx_libraries(cx),
                Target::Console => {
                    window.dispatch_action(Box::new(workspace::NewTerminal::default()), cx)
                }
                _ => {}
            }
        }
        cx.notify();
        true
    }

    fn load_zedx_scratches(
        &mut self,
        create_file: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.zedx_groups.scratch_loading || !self.project.read(cx).is_local() {
            return;
        }
        self.zedx_groups.scratch_loading = true;
        self.zedx_groups.scratch_error = None;
        let fs = self.fs.clone();
        let project = self.project.clone();
        let directory = paths::data_dir().join("scratches");
        cx.spawn_in(window, async move |this, cx| {
            let result: Result<()> = async {
                fs.create_dir(&directory).await?;
                let file = if create_file {
                    let mut path = None;
                    for index in 1..=10_000 {
                        let candidate = directory.join(format!("scratch-{index}.txt"));
                        if fs.metadata(&candidate).await?.is_none() {
                            fs.create_file(&candidate, fs::CreateOptions::default())
                                .await?;
                            path = Some(candidate);
                            break;
                        }
                    }
                    Some(path.context("No available scratch file name")?)
                } else {
                    None
                };
                let worktree = project
                    .update(cx, |project, cx| {
                        project.find_or_create_worktree(&directory, false, cx)
                    })
                    .await?;
                this.update(cx, |this, cx| {
                    this.zedx_groups.scratch = Some(worktree);
                    cx.notify();
                })?;
                if let Some(file) = file {
                    let workspace = this
                        .read_with(cx, |this, _| this.workspace.upgrade())?
                        .context("Workspace closed")?;
                    workspace
                        .update_in(cx, |workspace, window, cx| {
                            workspace.open_abs_path(file, OpenOptions::default(), window, cx)
                        })?
                        .await?;
                }
                Ok(())
            }
            .await;
            this.update(cx, |this, cx| {
                this.zedx_groups.scratch_loading = false;
                if let Err(error) = result {
                    this.zedx_groups.scratch_error = Some(error.to_string());
                }
                cx.notify();
            })
            .log_err();
        })
        .detach();
    }

    pub(super) fn render_zedx_group(&self, index: usize, cx: &mut Context<Self>) -> Stateful<Div> {
        let row = self.zedx_groups.rows.get(index).cloned();
        let Some(row) = row else {
            return div().id(("zedx-project-group-missing", index));
        };
        let expanded = self.zedx_groups.expanded.contains(&row.key);
        let selected = self.zedx_groups.selected.as_ref() == Some(&row.key);
        let editing = self
            .zedx_scratch_entry(&row.target, cx)
            .is_some_and(|entry| {
                self.state.edit_state.as_ref().is_some_and(|edit| {
                    !edit.is_new_entry()
                        && edit.worktree_id == entry.worktree_id
                        && edit.entry_id == entry.entry_id
                })
            });
        let message = matches!(row.target, Target::Message);
        let spacing = match ProjectPanelSettings::get_global(cx).entry_spacing {
            ProjectPanelEntrySpacing::Comfortable => ListItemSpacing::Dense,
            ProjectPanelEntrySpacing::Standard => ListItemSpacing::ExtraDense,
        };
        let chevron = if row.expandable() {
            Some(if expanded {
                ui::zedx::Icon::Down
            } else {
                ui::zedx::Icon::Chevron
            })
        } else {
            None
        };
        let scratch_menu = self.is_zedx_scratch_target(&row.target, cx);
        let selector = format!("zedx-project-{}", row.key);
        div()
            .id(("zedx-project-group", index))
            .debug_selector(move || selector)
            .when(selected, |row| row.bg(cx.theme().colors().element_selected))
            .child(
                ListItem::new(("zedx-project-group-item", index))
                    .spacing(spacing)
                    .indent_level(row.depth)
                    .indent_step_size(px(ProjectPanelSettings::get_global(cx).indent_size))
                    .selectable(false)
                    .disabled(message)
                    .start_slot(
                        h_flex()
                            .gap(px(4.0))
                            .child(div().w(px(12.0)).children(
                                chevron.map(|icon| ui::zedx::icon(icon, cx).size(px(12.0))),
                            ))
                            .child(ui::zedx::icon(row.glyph, cx).size(px(16.0))),
                    )
                    .tooltip(Tooltip::text(row.label.clone()))
                    .child(if editing {
                        h_flex()
                            .h_6()
                            .w_full()
                            .child(self.filename_editor.clone())
                            .into_any_element()
                    } else {
                        Label::new(row.label)
                            .truncate()
                            .size(ui::zedx::label_size())
                            .color(if message {
                                Color::Muted
                            } else {
                                Color::Default
                            })
                            .into_any_element()
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if editing {
                            return;
                        }
                        this.select_zedx_group(index, window, cx);
                        this.activate_zedx_group(None, window, cx);
                    }))
                    .when(scratch_menu, |item| {
                        item.on_secondary_mouse_down(cx.listener(
                            move |this, event: &MouseDownEvent, window, cx| {
                                cx.stop_propagation();
                                this.select_zedx_group(index, window, cx);
                                this.open_zedx_scratch_menu(event.position, window, cx);
                            },
                        ))
                    }),
            )
    }
}

#[cfg(test)]
#[path = "zedx_project_groups_tests.rs"]
mod tests;

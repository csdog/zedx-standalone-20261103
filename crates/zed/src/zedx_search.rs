use command_palette_hooks::CommandPaletteFilter;
use editor::{Bias, Editor, SelectionEffects, scroll::Autoscroll};
use gpui::{
    Action, App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, KeyBinding,
    Render, Task, TaskExt, WeakEntity, Window,
};
use language::{Anchor, Buffer, SymbolKind};
use picker::{Picker, PickerDelegate, PreviewUpdate};
use project::{
    PathMatchCandidateSet, Project, ProjectPath, Symbol, WorktreeId,
    lsp_store::SymbolLocation,
    search::{SearchQuery, SearchResult},
};
use std::{
    cell::RefCell,
    ops::Range,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use ui::{Checkbox, ContextMenu, ListItem, ListItemSpacing, PopoverMenu, Tooltip, prelude::*};
use util::{ResultExt, rel_path::RelPath};
use workspace::{ModalView, Workspace};

#[path = "zedx_gesture.rs"]
mod gesture;
#[path = "zedx_search_paths.rs"]
mod paths;
#[path = "zedx_search_policy.rs"]
mod policy;
#[path = "zedx_search_view.rs"]
mod view;
use policy::{FileFilter, FileTypes, action_search_text, highlight_indices};

use workspace::zedx_actions::{
    NextSearchCategory, PreviousSearchCategory, SearchActions, SearchEverywhere, SearchFiles,
};

#[derive(Clone, Copy, Default, PartialEq)]
enum Category {
    #[default]
    All,
    Classes,
    Files,
    Symbols,
    Actions,
    Text,
}
impl Category {
    const ALL: [Self; 6] = [
        Self::All,
        Self::Classes,
        Self::Files,
        Self::Symbols,
        Self::Actions,
        Self::Text,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Classes => "Classes",
            Self::Files => "Files",
            Self::Symbols => "Symbols",
            Self::Actions => "Actions",
            Self::Text => "Text",
        }
    }
}

pub(crate) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(
            "cmd-shift-f",
            workspace::zedx_actions::FindInFiles::default(),
            Some("Workspace"),
        ),
        KeyBinding::new(
            "cmd-shift-f",
            workspace::zedx_actions::FindInFiles::default(),
            Some("Workspace > Editor"),
        ),
        KeyBinding::new(
            "ctrl-shift-f",
            workspace::zedx_actions::FindInFiles::default(),
            Some("Workspace"),
        ),
        KeyBinding::new(
            "ctrl-shift-f",
            workspace::zedx_actions::FindInFiles::default(),
            Some("Workspace > Editor"),
        ),
        KeyBinding::new("cmd-p", SearchFiles, Some("Workspace")),
        KeyBinding::new("cmd-p", SearchFiles, Some("Workspace > Editor")),
        KeyBinding::new("ctrl-p", SearchFiles, Some("Workspace")),
        KeyBinding::new("ctrl-p", SearchFiles, Some("Workspace > Editor")),
        KeyBinding::new(
            "shift-enter",
            menu::SecondaryConfirm,
            Some("ZedXSearch > Editor"),
        ),
        KeyBinding::new("cmd-k", SearchEverywhere, Some("Workspace")),
        KeyBinding::new("cmd-k", SearchEverywhere, Some("Workspace > Editor")),
        KeyBinding::new("tab", NextSearchCategory, Some("ZedXSearch > Editor")),
        KeyBinding::new(
            "shift-tab",
            PreviousSearchCategory,
            Some("ZedXSearch > Editor"),
        ),
    ]);
}

pub fn init(cx: &mut App) {
    workspace::zedx_actions::init(cx);
    cx.observe_new(
        |workspace: &mut Workspace, window, cx: &mut Context<Workspace>| {
            workspace.register_action(|workspace, _: &SearchEverywhere, window, cx| {
                open(workspace, window, cx)
            });
            workspace.register_action(|workspace, _: &SearchFiles, window, cx| {
                open_category(workspace, Some(Category::Files), window, cx)
            });
            workspace.register_action(|workspace, _: &SearchActions, window, cx| {
                open_category(workspace, Some(Category::Actions), window, cx)
            });
            let gesture = Rc::new(RefCell::new(gesture::DoubleShift::default()));
            let clock = cx.background_executor().now();
            if let Some(window) = window {
                let gesture = gesture.clone();
                cx.observe_window_activation(window, move |_, window, _| {
                    if !window.is_window_active() {
                        *gesture.borrow_mut() = gesture::DoubleShift::default();
                    }
                })
                .detach();
            }
            workspace.register_action_renderer(move |div, _, _, cx| {
                let on_key = gesture.clone();
                let on_modifiers = gesture.clone();
                div.capture_key_down(move |_, _, _| on_key.borrow_mut().reset())
                    .on_modifiers_changed(cx.listener(
                        move |workspace, event: &gpui::ModifiersChangedEvent, window, cx| {
                            let modifiers = event.modifiers;
                            let triggered = on_modifiers.borrow_mut().modifiers_changed(
                                modifiers.shift,
                                modifiers.control
                                    || modifiers.alt
                                    || modifiers.platform
                                    || modifiers.function,
                                cx.background_executor().now().duration_since(clock),
                            );
                            if triggered {
                                open(workspace, window, cx);
                            }
                        },
                    ))
            });
            cx.notify();
        },
    )
    .detach();
}

fn open(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    open_category(workspace, None, window, cx);
}

fn open_category(
    workspace: &mut Workspace,
    category: Option<Category>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    if let Some(modal) = workspace.active_modal::<SearchWindow>(cx) {
        modal.update(cx, |modal, cx| {
            modal.picker.update(cx, |picker, cx| {
                if let Some(category) = category {
                    picker.delegate.category = category;
                } else {
                    picker.delegate.include_non_project = !picker.delegate.include_non_project;
                }
                picker.update_matches(picker.query(cx), window, cx);
            });
            window.focus(&modal.picker.focus_handle(cx), cx);
            cx.notify();
        });
        return;
    }
    let project = workspace.project().clone();
    let recent = workspace
        .recent_navigation_history(Some(50), cx)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    let previous_focus = window.focused(cx);
    let filter = CommandPaletteFilter::try_global(cx);
    let actions = window
        .available_actions(cx)
        .into_iter()
        .filter(|action| !filter.is_some_and(|filter| filter.is_hidden(&**action)))
        .map(|action| (command_palette::humanize_action_name(action.name()), action))
        .collect();
    let workspace_handle = cx.entity().downgrade();
    workspace.toggle_modal(window, cx, move |window, cx| {
        let delegate = SearchDelegate {
            workspace: workspace_handle,
            project: project.clone(),
            recent,
            actions,
            previous_focus,
            category: category.unwrap_or_default(),
            include_non_project: false,
            file_filter: FileTypes::default(),
            rows: Vec::new(),
            selected: 0,
            generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            error: None,
            query: String::new(),
        };
        let preview = picker_preview::editor_preview(project, window, cx);
        let picker = cx.new(|cx| {
            Picker::list_with_preview(delegate, preview, window, cx)
                .list_measure_all()
                .embedded()
                .initial_width(rems(55.0))
                .max_height(rems(32.0))
                .show_scrollbar(true)
        });
        cx.subscribe(&picker, |_, _, _: &DismissEvent, cx| cx.emit(DismissEvent))
            .detach();
        cx.observe(&picker, |_, _, cx| cx.notify()).detach();
        SearchWindow { picker }
    });
}

struct SearchWindow {
    picker: Entity<Picker<SearchDelegate>>,
}
impl ModalView for SearchWindow {
    fn render_bare(&self) -> bool {
        true
    }
}
impl EventEmitter<DismissEvent> for SearchWindow {}
impl Focusable for SearchWindow {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}
impl SearchWindow {
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.picker.update(cx, |picker, cx| {
            picker.update_matches(picker.query(cx), window, cx)
        });
        window.focus(&self.picker.focus_handle(cx), cx);
        cx.notify();
    }

    fn find_in_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picker = self.picker.read(cx);
        if picker.delegate.category == Category::Actions {
            return;
        }
        let workspace = picker.delegate.workspace.clone();
        let action = workspace::zedx_actions::FindInFiles {
            query: Some(picker.query(cx)),
            include_ignored: Some(picker.delegate.include_non_project),
        };
        cx.emit(DismissEvent);
        workspace
            .update(cx, |workspace, cx| {
                window.focus(&workspace.focus_handle(cx), cx);
                window.dispatch_action(action.boxed_clone(), cx);
            })
            .log_err();
    }

    fn cycle(&mut self, reverse: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.picker.update(cx, |picker, cx| {
            let index = Category::ALL
                .iter()
                .position(|category| *category == picker.delegate.category)
                .unwrap_or(0);
            let offset = if reverse { Category::ALL.len() - 1 } else { 1 };
            picker.delegate.category = Category::ALL[(index + offset) % Category::ALL.len()];
            picker.update_matches(picker.query(cx), window, cx);
        });
    }
}
#[derive(Clone)]
enum Target {
    File(ProjectPath),
    Symbol(Box<Symbol>),
    Action(usize),
    Text(Entity<Buffer>, Range<Anchor>),
}
#[derive(Clone)]
struct Row {
    label: String,
    detail: String,
    target: Target,
}
struct SearchDelegate {
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    recent: Vec<ProjectPath>,
    actions: Vec<(String, Box<dyn Action>)>,
    previous_focus: Option<FocusHandle>,
    category: Category,
    include_non_project: bool,
    file_filter: FileTypes,
    rows: Vec<Row>,
    selected: usize,
    generation: u64,
    cancel: Arc<AtomicBool>,
    error: Option<String>,
    query: String,
}
impl Drop for SearchDelegate {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}
impl SearchDelegate {
    fn file_row(path: ProjectPath, project: &Project, cx: &App) -> Row {
        let absolute = project.absolute_path(&path, cx);
        let label = path
            .path
            .file_name()
            .map(str::to_string)
            .or_else(|| absolute.as_ref()?.file_name()?.to_str().map(str::to_string))
            .unwrap_or_else(|| "Project file".into());
        let detail = if path.path.is_empty() {
            label.clone()
        } else {
            path.path.as_unix_str().to_string()
        };
        Row {
            label,
            detail,
            target: Target::File(path),
        }
    }
    fn publish(
        picker: &mut Picker<Self>,
        generation: u64,
        rows: Vec<Row>,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) {
        let delegate = &mut picker.delegate;
        if delegate.generation != generation {
            return;
        }
        let selected = delegate.rows.get(delegate.selected).cloned();
        delegate.rows.extend(rows);
        delegate.rows.sort_by_key(|row| row.group().0);
        if let Some(selected) = selected {
            delegate.selected = delegate
                .rows
                .iter()
                .position(|row| row.same_target(&selected))
                .unwrap_or(0);
        }
        picker.zedx_streamed_matches_changed(window, cx);
    }
    fn search_text(
        &mut self,
        query: String,
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        let query = match SearchQuery::text(
            query,
            false,
            false,
            self.include_non_project,
            Default::default(),
            Default::default(),
            false,
            None,
        ) {
            Ok(query) => query,
            Err(error) => {
                self.error = Some(format!("Text search: {error}"));
                cx.notify();
                return Task::ready(());
            }
        };
        let file_filter = self.file_filter;
        let include_non_project = self.include_non_project;
        let project = self.project.clone();
        let results = self
            .project
            .update(cx, |project, cx| project.search(query, cx));
        cx.spawn_in(window, async move |picker, cx| {
            let _search_task = results.task_handle;
            let mut count = 0;
            while let Ok(result) = results.rx.recv().await {
                if let SearchResult::Buffer { buffer, ranges } = result {
                    let accepted = buffer.read_with(cx, |buffer, cx| {
                        buffer.file().is_some_and(|file| {
                            file_filter.accepts(&file.full_path(cx).to_string_lossy())
                                && (include_non_project
                                    || project
                                        .read(cx)
                                        .worktree_for_id(file.worktree_id(cx), cx)
                                        .is_some_and(|worktree| worktree.read(cx).is_visible()))
                        })
                    });
                    if !accepted {
                        continue;
                    }
                    let rows = ranges
                        .into_iter()
                        .take(100 - count)
                        .map(|range| {
                            let (label, detail) = buffer.read_with(cx, |buffer, cx| {
                                let snapshot = buffer.snapshot();
                                let point: language::Point =
                                    snapshot.summary_for_anchor(&range.start);
                                let end = if point.row < snapshot.max_point().row {
                                    language::Point::new(point.row + 1, 0)
                                } else {
                                    snapshot.max_point()
                                };
                                let label = snapshot
                                    .text_for_range(language::Point::new(point.row, 0)..end)
                                    .flat_map(str::chars)
                                    .take(240)
                                    .collect::<String>()
                                    .trim()
                                    .to_string();
                                let path = buffer
                                    .file()
                                    .map(|file| file.full_path(cx).to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                (label, format!("{}:{}", path, point.row + 1))
                            });
                            Row {
                                label,
                                detail,
                                target: Target::Text(buffer.clone(), range),
                            }
                        })
                        .collect::<Vec<_>>();
                    count += rows.len();
                    if picker
                        .update_in(cx, |picker, window, cx| {
                            Self::publish(picker, generation, rows, window, cx)
                        })
                        .is_err()
                        || count >= 100
                    {
                        break;
                    }
                }
            }
        })
    }
    fn open_buffer(
        &self,
        buffer: Task<anyhow::Result<Entity<Buffer>>>,
        symbol: Option<Symbol>,
        range: Option<Range<Anchor>>,
        secondary: bool,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) {
        let workspace = self.workspace.clone();
        cx.spawn_in(window, async move |_, cx| {
            let buffer = buffer.await?;
            workspace.update_in(cx, |workspace, window, cx| {
                let position = symbol.map(|symbol| {
                    buffer
                        .read(cx)
                        .clip_point_utf16(symbol.range.start, Bias::Left)
                });
                let pane = secondary.then(|| workspace.adjacent_pane(window, cx));
                let editor = workspace
                    .open_project_item::<Editor>(pane, buffer, true, true, true, true, window, cx);
                editor.update(cx, |editor, cx| {
                    let snapshot = editor.buffer().read(cx).snapshot(cx);
                    let Some(buffer) = snapshot.as_singleton() else {
                        return;
                    };
                    let range = range.or_else(|| {
                        position.map(|position| {
                            let anchor = buffer.anchor_before(position);
                            anchor..anchor
                        })
                    });
                    let Some(range) = range else {
                        return;
                    };
                    let Some(start) = snapshot.anchor_in_buffer(range.start) else {
                        return;
                    };
                    let Some(end) = snapshot.anchor_in_buffer(range.end) else {
                        return;
                    };
                    editor.change_selections(
                        SelectionEffects::scroll(Autoscroll::center()).unfold(),
                        window,
                        cx,
                        |selections| selections.select_ranges([start..end]),
                    );
                });
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }
}
impl PickerDelegate for SearchDelegate {
    type ListItem = AnyElement;
    fn name() -> &'static str {
        "ZedX Search Everywhere"
    }
    fn placeholder_text(&self, _: &mut Window, _: &mut App) -> Arc<str> {
        "Search Everywhere".into()
    }
    fn match_count(&self) -> usize {
        self.rows.len()
    }
    fn selected_index(&self) -> usize {
        self.selected
    }
    fn set_selected_index(&mut self, index: usize, _: &mut Window, _: &mut Context<Picker<Self>>) {
        self.selected = index;
    }
    fn dismissed(&mut self, _: &mut Window, _: &mut Context<Picker<Self>>) {
        self.cancel.store(true, Ordering::Release);
    }
    fn render_footer(
        &self,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<AnyElement> {
        Some(self.render_source_footer(window, cx))
    }
    fn render_match(
        &self,
        index: usize,
        selected: bool,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<AnyElement> {
        self.render_row(index, selected, window, cx)
    }
    fn try_get_preview_data_for_match(&self, cx: &App) -> Option<PreviewUpdate> {
        match &self.rows.get(self.selected)?.target {
            Target::File(path) => Some(PreviewUpdate::from_path(
                self.project.read(cx).absolute_path(path, cx)?,
            )),
            Target::Symbol(symbol) => Some(PreviewUpdate::from_symbol((**symbol).clone())),
            Target::Text(buffer, range) => {
                let snapshot = buffer.read(cx).snapshot();
                Some(PreviewUpdate::from_buffer(
                    buffer.clone(),
                    picker::MatchLocation {
                        anchor_range: range.clone(),
                        range: snapshot.offset_for_anchor(&range.start)
                            ..snapshot.offset_for_anchor(&range.end),
                    },
                ))
            }
            Target::Action(index) => {
                let (name, _) = self.actions.get(*index)?;
                let mut message = picker::HighlightedTextBuilder::default();
                message.push_plain(format!("Run {name}"));
                Some(PreviewUpdate::message(message.build()))
            }
        }
    }
    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(row) = self.rows.get(self.selected).cloned() else {
            return;
        };
        cx.emit(DismissEvent);
        match row.target {
            Target::File(path) => {
                self.workspace
                    .update(cx, |workspace, cx| {
                        let pane =
                            secondary.then(|| workspace.adjacent_pane(window, cx).downgrade());
                        workspace
                            .open_path(path, pane, true, window, cx)
                            .detach_and_log_err(cx)
                    })
                    .log_err();
            }
            Target::Symbol(symbol) => {
                let buffer = self.project.update(cx, |project, cx| {
                    project.open_buffer_for_symbol(&symbol, cx)
                });
                self.open_buffer(buffer, Some(*symbol), None, secondary, window, cx);
            }
            Target::Text(buffer, range) => self.open_buffer(
                Task::ready(Ok(buffer)),
                None,
                Some(range),
                secondary,
                window,
                cx,
            ),
            Target::Action(index) => {
                if let Some(focus) = &self.previous_focus {
                    window.focus(focus, cx);
                }
                if let Some((_, action)) = self.actions.get(index) {
                    window.dispatch_action(action.boxed_clone(), cx);
                }
            }
        }
    }
    fn update_matches(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        let query = query.trim();
        let (category, query) = if matches!(self.category, Category::All | Category::Actions) {
            query
                .strip_prefix('/')
                .map(|query| (Category::Actions, query.trim()))
                .unwrap_or((self.category, query))
        } else {
            (self.category, query)
        };
        let query = query.to_string();
        self.query = query.clone();
        self.generation += 1;
        let generation = self.generation;
        self.cancel.store(true, Ordering::Release);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.rows.clear();
        self.selected = 0;
        self.error = None;
        if query.is_empty() && category != Category::Actions {
            if matches!(category, Category::All | Category::Files) {
                self.rows = self
                    .recent
                    .iter()
                    .filter(|path| {
                        let Some(worktree) =
                            self.project.read(cx).worktree_for_id(path.worktree_id, cx)
                        else {
                            return false;
                        };
                        let worktree = worktree.read(cx);
                        let snapshot = worktree.snapshot();
                        let name = if path.path.is_empty() {
                            snapshot.root_name().as_unix_str()
                        } else {
                            path.path.as_unix_str()
                        };
                        self.file_filter.accepts(name)
                            && (self.include_non_project
                                || (worktree.is_visible()
                                    && snapshot
                                        .entry_for_path(&path.path)
                                        .is_none_or(|entry| !entry.is_ignored)))
                    })
                    .cloned()
                    .map(|path| Self::file_row(path, self.project.read(cx), cx))
                    .collect();
            }
            cx.notify();
            return Task::ready(());
        }
        let text_task = matches!(category, Category::All | Category::Text)
            .then(|| self.search_text(query.clone(), generation, window, cx));
        if category == Category::Text {
            return text_task.unwrap_or_else(|| Task::ready(()));
        }
        let sets = if matches!(category, Category::All | Category::Files) {
            self.project
                .read(cx)
                .worktree_store()
                .read(cx)
                .worktrees()
                .filter(|worktree| self.include_non_project || worktree.read(cx).is_visible())
                .map(|worktree| PathMatchCandidateSet {
                    snapshot: worktree.read(cx).snapshot(),
                    include_ignored: self.include_non_project,
                    include_root_name: false,
                    candidates: project::Candidates::Files,
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let symbols = matches!(
            category,
            Category::All | Category::Classes | Category::Symbols
        )
        .then(|| {
            self.project
                .update(cx, |project, cx| project.symbols(&query, cx))
        });
        let commands = if matches!(category, Category::All | Category::Actions) {
            self.actions
                .iter()
                .enumerate()
                .map(|(index, (name, action))| {
                    fuzzy_nucleo::StringMatchCandidate::new(
                        index,
                        action_search_text(action.name(), name),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        let cancel = self.cancel.clone();
        let file_filter = self.file_filter;
        let include_non_project = self.include_non_project;
        cx.spawn_in(window, async move |picker, cx| {
            let executor = cx.background_executor().clone();
            let sets = cx
                .background_spawn(async move {
                    sets.into_iter()
                        .map(|set| paths::Candidates::new(set, file_filter))
                        .collect::<Vec<_>>()
                })
                .await;
            let paths = fuzzy_nucleo::match_path_sets(
                &sets,
                &query,
                &None,
                fuzzy_nucleo::Case::Ignore,
                100,
                &cancel,
                executor.clone(),
            )
            .await;
            let commands = fuzzy_nucleo::match_strings_async(
                &commands,
                &query,
                fuzzy_nucleo::Case::Smart,
                fuzzy_nucleo::LengthPenalty::On,
                30,
                &cancel,
                executor,
            )
            .await;
            if cancel.load(Ordering::Acquire) {
                return;
            }
            picker
                .update_in(cx, |picker, window, cx| {
                    let project = picker.delegate.project.read(cx);
                    let mut rows = paths
                        .into_iter()
                        .map(|path| {
                            let worktree_id = WorktreeId::from_usize(path.worktree_id);
                            let path = if project
                                .worktree_for_id(worktree_id, cx)
                                .is_some_and(|worktree| worktree.read(cx).is_single_file())
                            {
                                RelPath::empty_arc()
                            } else {
                                path.path
                            };
                            Self::file_row(ProjectPath { worktree_id, path }, project, cx)
                        })
                        .collect::<Vec<_>>();
                    rows.extend(commands.into_iter().filter_map(|command| {
                        let (label, _) = picker.delegate.actions.get(command.candidate_id)?;
                        Some(Row {
                            label: label.clone(),
                            detail: String::new(),
                            target: Target::Action(command.candidate_id),
                        })
                    }));
                    Self::publish(picker, generation, rows, window, cx);
                })
                .log_err();
            if let Some(symbols) = symbols {
                match symbols.await {
                    Ok(symbols) => {
                        let rows = symbols
                            .into_iter()
                            .filter(|symbol| {
                                category != Category::Classes
                                    || matches!(
                                        symbol.kind,
                                        SymbolKind::Class
                                            | SymbolKind::Struct
                                            | SymbolKind::Interface
                                            | SymbolKind::Enum
                                    )
                            })
                            .map(|symbol| {
                                let path = match &symbol.path {
                                    SymbolLocation::InProject(path) => {
                                        path.path.as_unix_str().to_string()
                                    }
                                    SymbolLocation::OutsideProject { abs_path, .. } => {
                                        abs_path.to_string_lossy().into_owned()
                                    }
                                };
                                Row {
                                    label: symbol.name.clone(),
                                    detail: format!("{}:{}", path, symbol.range.start.0.row + 1),
                                    target: Target::Symbol(Box::new(symbol)),
                                }
                            })
                            .collect::<Vec<_>>();
                        picker
                            .update_in(cx, |picker, window, cx| {
                                let rows = rows
                                    .into_iter()
                                    .filter(|row: &Row| {
                                        let Target::Symbol(symbol) = &row.target else {
                                            return true;
                                        };
                                        match &symbol.path {
                                            SymbolLocation::InProject(path) => {
                                                file_filter.accepts(path.path.as_unix_str())
                                                    && (include_non_project
                                                        || picker
                                                            .delegate
                                                            .project
                                                            .read(cx)
                                                            .worktree_for_id(path.worktree_id, cx)
                                                            .is_some_and(|worktree| {
                                                                worktree.read(cx).is_visible()
                                                            }))
                                            }
                                            SymbolLocation::OutsideProject { abs_path, .. } => {
                                                include_non_project
                                                    && file_filter
                                                        .accepts(&abs_path.to_string_lossy())
                                            }
                                        }
                                    })
                                    .take(100)
                                    .collect();
                                Self::publish(picker, generation, rows, window, cx)
                            })
                            .log_err();
                    }
                    Err(error) => {
                        picker
                            .update(cx, |picker, cx| {
                                if picker.delegate.generation == generation {
                                    picker.delegate.error =
                                        Some(format!("Language server: {error}"));
                                    cx.notify();
                                }
                            })
                            .log_err();
                    }
                }
            }
            if let Some(text_task) = text_task {
                text_task.await;
            }
        })
    }
}

#[cfg(test)]
#[path = "zedx_search_tests.rs"]
pub(crate) mod tests;

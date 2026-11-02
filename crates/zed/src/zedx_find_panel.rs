use super::*;
use gpui::{KeyBinding, ScrollStrategy, UniformListScrollHandle};
use project::search::{SearchQuery, SearchResult};
use ui::HighlightedLabel;
use workspace::zedx_actions::FindInFiles;

#[derive(Clone)]
struct Match {
    buffer: Entity<Buffer>,
    range: Range<Anchor>,
    location: String,
    path: String,
    point: language::Point,
    highlight: Range<usize>,
    text: String,
}

#[derive(Clone)]
enum Entry {
    File { path: String, count: usize },
    Match(usize),
}

pub(crate) struct Find {
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    focus: FocusHandle,
    zoomed: bool,
    query: Entity<Editor>,
    last_query: String,
    include_ignored: bool,
    rows: Vec<Match>,
    entries: Vec<Entry>,
    selected: usize,
    searching: bool,
    error: Option<String>,
    search: Task<()>,
    scroll: UniformListScrollHandle,
    _subscription: Subscription,
}
impl Find {
    pub(super) fn new(workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Search in project…", window, cx);
            editor
        });
        let subscription = cx.observe_in(&query, window, |this: &mut Self, _, window, cx| {
            this.refresh(false, window, cx);
        });
        Self {
            workspace: workspace.weak_handle(),
            project: workspace.project().clone(),
            focus: cx.focus_handle(),
            zoomed: false,
            query,
            last_query: String::new(),
            include_ignored: false,
            rows: Vec::new(),
            entries: Vec::new(),
            selected: 0,
            searching: false,
            error: None,
            search: Task::ready(()),
            scroll: UniformListScrollHandle::new(),
            _subscription: subscription,
        }
    }
    pub(super) fn configure(
        &mut self,
        action: &FindInFiles,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(include) = action.include_ignored {
            self.include_ignored = include;
        }
        if let Some(query) = &action.query {
            self.query
                .update(cx, |editor, cx| editor.set_text(query.clone(), window, cx));
        }
        self.refresh(true, window, cx);
    }
    fn activation_focus(&self, cx: &App) -> FocusHandle {
        self.query.focus_handle(cx)
    }
    fn on_active(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) {}
    fn badge(&self, _: &App) -> Option<String> {
        (!self.rows.is_empty()).then(|| self.rows.len().to_string())
    }
    fn refresh(&mut self, force: bool, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.query.read(cx).text(cx);
        if !force && text == self.last_query {
            return;
        }
        self.last_query = text.clone();
        self.search = Task::ready(());
        self.rows.clear();
        self.entries.clear();
        self.selected = 0;
        self.error = None;
        self.searching = !text.is_empty();
        cx.notify();
        if text.is_empty() {
            return;
        }
        let query = match SearchQuery::text(
            text,
            false,
            false,
            self.include_ignored,
            Default::default(),
            Default::default(),
            false,
            None,
        ) {
            Ok(query) => query,
            Err(error) => {
                self.error = Some(error.to_string());
                self.searching = false;
                return;
            }
        };
        let results = self
            .project
            .update(cx, |project, cx| project.search(query, cx));
        self.search = cx.spawn_in(window, async move |panel, cx| {
            let _search_task = results.task_handle;
            let mut count = 0;
            while let Ok(result) = results.rx.recv().await {
                if let SearchResult::Buffer { buffer, ranges } = result {
                    let rows = buffer.read_with(cx, |buffer_data, cx| {
                        let snapshot = buffer_data.snapshot();
                        let path = buffer_data
                            .file()
                            .map(|file| file.full_path(cx).to_string_lossy().into_owned())
                            .unwrap_or_default();
                        ranges
                            .into_iter()
                            .take(1000 - count)
                            .map(|range| {
                                let point: language::Point =
                                    snapshot.summary_for_anchor(&range.start);
                                let end = if point.row < snapshot.max_point().row {
                                    language::Point::new(point.row + 1, 0)
                                } else {
                                    snapshot.max_point()
                                };
                                let raw_text = snapshot
                                    .text_for_range(language::Point::new(point.row, 0)..end)
                                    .flat_map(str::chars)
                                    .take(240)
                                    .collect::<String>();
                                let leading = raw_text.len() - raw_text.trim_start().len();
                                let text = raw_text.trim().to_string();
                                let end: language::Point = snapshot.summary_for_anchor(&range.end);
                                let start = (point.column as usize)
                                    .saturating_sub(leading)
                                    .min(text.len());
                                let end = if end.row == point.row {
                                    (end.column as usize)
                                        .saturating_sub(leading)
                                        .min(text.len())
                                } else {
                                    text.len()
                                };
                                Match {
                                    buffer: buffer.clone(),
                                    range,
                                    path: path.clone(),
                                    point,
                                    highlight: start..end,
                                    location: format!(
                                        "{path}:{}:{}",
                                        point.row + 1,
                                        point.column + 1
                                    ),
                                    text,
                                }
                            })
                            .collect::<Vec<_>>()
                    });
                    count += rows.len();
                    if panel
                        .update(cx, |panel, cx| {
                            let selected = panel
                                .rows
                                .get(panel.selected)
                                .map(|row| (row.buffer.clone(), row.range.clone()));
                            panel.rows.extend(rows);
                            panel.rows.sort_by(|left, right| {
                                left.path
                                    .cmp(&right.path)
                                    .then_with(|| left.point.cmp(&right.point))
                            });
                            if let Some((buffer, range)) = selected {
                                panel.selected = panel
                                    .rows
                                    .iter()
                                    .position(|row| row.buffer == buffer && row.range == range)
                                    .unwrap_or(0);
                            }
                            panel.rebuild_entries();
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                    if count == 1000 {
                        break;
                    }
                }
            }
            panel
                .update(cx, |panel, cx| {
                    panel.searching = false;
                    cx.notify();
                })
                .log_err();
        });
    }
    fn rebuild_entries(&mut self) {
        self.entries.clear();
        for (index, row) in self.rows.iter().enumerate() {
            if index == 0
                || self
                    .rows
                    .get(index - 1)
                    .is_none_or(|previous| previous.path != row.path)
            {
                let count = self
                    .rows
                    .iter()
                    .skip(index)
                    .take_while(|next| next.path == row.path)
                    .count();
                self.entries.push(Entry::File {
                    path: row.path.clone(),
                    count,
                });
            }
            self.entries.push(Entry::Match(index));
        }
    }
    fn select(&mut self, previous: bool, cx: &mut Context<Self>) {
        let count = self.rows.len();
        if count == 0 {
            return;
        }
        self.selected = (self.selected + if previous { count - 1 } else { 1 }) % count;
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| matches!(entry, Entry::Match(index) if *index == self.selected))
        {
            self.scroll.scroll_to_item(index, ScrollStrategy::Center);
        }
        cx.notify();
    }
    fn open(&self, row: Match, window: &mut Window, cx: &mut Context<Self>) {
        super::open_source(&self.workspace, row.buffer, row.range, window, cx);
    }
}
fixed_panel!(
    Find,
    "Find",
    "ZedXFind",
    DockPosition::Left,
    265.0,
    IconName::MagnifyingGlass,
    FindInFiles::default(),
    2
);
impl Render for Find {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
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
                ui::zedx::tool_window_head("find", ui::zedx::Icon::FindTool, "Find in Files", cx)
                    .justify_between()
                    .child(
                        ui::zedx::tool_button(
                            "zedx-find-hide",
                            ui::zedx::Icon::Minus,
                            "Hide Find",
                            cx,
                        )
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Close))),
                    ),
            )
            .child(
                h_flex()
                    .px_2()
                    .py_1()
                    .gap_1()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Icon::new(IconName::MagnifyingGlass)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    )
                    .child(div().flex_1().min_w_0().child(self.query.clone()))
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
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.query.update(cx, |editor, cx| {
                                            editor.set_text("", window, cx)
                                        });
                                        window.focus(&this.query.focus_handle(cx), cx);
                                    },
                                )),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .px_2()
                    .py_1()
                    .justify_between()
                    .child(
                        Label::new(if self.searching {
                            "Searching…".to_string()
                        } else if self.rows.len() == 1000 {
                            "1000+ results".into()
                        } else {
                            format!(
                                "{} matches in {} files",
                                self.rows.len(),
                                self.entries
                                    .iter()
                                    .filter(|entry| matches!(entry, Entry::File { .. }))
                                    .count()
                            )
                        })
                        .size(ui::zedx::caption_size())
                        .color(Color::Muted),
                    )
                    .child(
                        Button::new(
                            "zedx-find-scope",
                            if self.include_ignored {
                                "All Places"
                            } else {
                                "Project"
                            },
                        )
                        .label_size(ui::zedx::caption_size())
                        .tooltip(Tooltip::text("Toggle ignored files"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.include_ignored = !this.include_ignored;
                            this.refresh(true, window, cx);
                        })),
                    ),
            )
            .when_some(self.error.clone(), |column, error| {
                column.child(Label::new(error).color(Color::Warning))
            })
            .when(self.rows.is_empty() && !self.searching, |column| {
                column.child(
                    div().p_2().child(
                        Label::new(if self.last_query.is_empty() {
                            "Search text across project files"
                        } else {
                            "No matches"
                        })
                        .size(ui::zedx::label_size())
                        .color(Color::Muted),
                    ),
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
                                            .h(px(26.0))
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
                                                    .size(ui::zedx::label_size())
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
                                                .height(px(26.0))
                                                .toggle(index == this.selected)
                                                .child(
                                                    h_flex()
                                                        .min_w_0()
                                                        .gap_2()
                                                        .child(
                                                            Label::new(
                                                                (row.point.row + 1).to_string(),
                                                            )
                                                            .size(ui::zedx::caption_size())
                                                            .color(Color::Muted),
                                                        )
                                                        .child(
                                                            HighlightedLabel::from_ranges(
                                                                row.text.clone(),
                                                                vec![row.highlight.clone()],
                                                            )
                                                            .size(ui::zedx::label_size())
                                                            .truncate(),
                                                        ),
                                                )
                                                .tooltip(Tooltip::text(row.location.clone()))
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
                .track_scroll(&self.scroll)
                .flex_1(),
            )
    }
}

pub(super) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", menu::SelectNext, Some("ZedXFind > Editor")),
        KeyBinding::new("up", menu::SelectPrevious, Some("ZedXFind > Editor")),
        KeyBinding::new("enter", menu::Confirm, Some("ZedXFind > Editor")),
    ]);
}

#[cfg(test)]
#[path = "zedx_find_panel_tests.rs"]
mod tests;

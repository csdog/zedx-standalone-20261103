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
    DockPosition::Bottom,
    202.0,
    IconName::MagnifyingGlass,
    FindInFiles::default(),
    2
);
#[path = "zedx_find_view.rs"]
mod view;

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

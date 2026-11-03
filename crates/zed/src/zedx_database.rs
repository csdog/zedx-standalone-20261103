use super::*;
#[path = "zedx_database_session.rs"]
mod session;
use db::sqlez::inspect::{self, QueryResult};
use gpui::{KeyBinding, PathPromptOptions, ScrollHandle};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use workspace::zedx_actions::ShowDatabase;

gpui::actions!(zedx_database, [ExecuteQuery]);
const SCHEMA: &str = "SELECT name, type FROM sqlite_schema WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name";

pub(crate) struct Database {
    project: Entity<Project>,
    session: session::Session,
    restoring_session: bool,
    _query_subscription: Subscription,
    focus: FocusHandle,
    zoomed: bool,
    path: Option<PathBuf>,
    query: Entity<Editor>,
    schema: Vec<String>,
    results: QueryResult,
    error: Option<String>,
    pending: bool,
    generation: u64,
    cancel: Arc<AtomicBool>,
    task: Task<()>,
    scroll: ScrollHandle,
}
impl Drop for Database {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Database {
    pub(super) fn new(workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (session, restore_error) = session::Session::load(workspace, cx);
        let query = cx.new(|cx| {
            let mut editor = Editor::multi_line(window, cx);
            editor.set_placeholder_text("SELECT * FROM table_name", window, cx);
            editor.set_text(session.saved.draft.clone(), window, cx);
            editor
        });
        let query_subscription = cx.subscribe(&query, |this: &mut Self, _, event, cx| {
            if matches!(event, editor::EditorEvent::BufferEdited) {
                this.persist_session(cx);
            }
        });
        let path = session.saved.selected.clone();
        Self {
            restoring_session: path.is_some(),
            session,
            _query_subscription: query_subscription,
            project: workspace.project().clone(),
            focus: cx.focus_handle(),
            zoomed: false,
            path,
            query,
            schema: Vec::new(),
            results: QueryResult::default(),
            error: restore_error,
            pending: false,
            generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            task: Task::ready(()),
            scroll: ScrollHandle::new(),
        }
    }
    fn activation_focus(&self, cx: &App) -> FocusHandle {
        self.query.focus_handle(cx)
    }
    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        if active && self.restoring_session {
            self.restoring_session = false;
            self.execute(true, window, cx);
        }
    }
    fn badge(&self, _: &App) -> Option<String> {
        None
    }
    fn disconnect(&mut self, cx: &mut Context<Self>) {
        self.cancel.store(true, Ordering::Relaxed);
        self.generation += 1;
        self.task = Task::ready(());
        self.path = None;
        self.restoring_session = false;
        self.session.saved.selected = None;
        self.schema.clear();
        self.results = QueryResult::default();
        self.pending = false;
        self.error = None;
        self.persist_session(cx);
        cx.notify();
    }
    fn choose_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.project.read(cx).is_local() {
            self.error = Some("SQLite browsing is available for local workspaces.".into());
            cx.notify();
            return;
        }
        let generation = self.generation;
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open SQLite database (read only)".into()),
        });
        cx.spawn_in(window, async move |panel, cx| {
            let result: anyhow::Result<Option<PathBuf>> =
                async { Ok(prompt.await??.and_then(|paths| paths.into_iter().next())) }.await;
            panel
                .update_in(cx, |panel, window, cx| {
                    if panel.generation != generation {
                        return;
                    }
                    match result {
                        Ok(Some(path)) => {
                            panel.disconnect(cx);
                            panel.path = Some(path);
                            panel.execute(true, window, cx);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            panel.error = Some(error.to_string());
                            cx.notify();
                        }
                    }
                })
                .log_err();
        })
        .detach();
    }
    fn execute(&mut self, schema: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            return;
        };
        self.cancel.store(true, Ordering::Relaxed);
        self.generation += 1;
        let generation = self.generation;
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let sql = if schema {
            SCHEMA.to_string()
        } else {
            self.query.read(cx).text(cx)
        };
        let history_sql = (!schema).then(|| sql.clone());
        self.pending = true;
        self.error = None;
        self.task = cx.spawn_in(window, async move |panel, cx| {
            let result = cx
                .background_spawn(async move { inspect::query(&path, &sql, cancel) })
                .await;
            panel
                .update_in(cx, |panel, _, cx| {
                    if panel.generation != generation {
                        return;
                    }
                    panel.pending = false;
                    match result {
                        Ok(result) if schema => {
                            panel.schema = result
                                .rows
                                .into_iter()
                                .filter_map(|row| row.into_iter().next())
                                .collect();
                            panel.connected_session(cx);
                        }
                        Ok(result) => {
                            panel.results = result;
                            panel.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
                            if let Some(sql) = history_sql {
                                panel.completed_query(sql, cx);
                            }
                        }
                        Err(error) => panel.error = Some(error.to_string()),
                    }
                    cx.notify();
                })
                .log_err();
        });
        cx.notify();
    }
}
fixed_panel!(
    Database,
    "Database",
    "ZedXDatabase",
    DockPosition::Right,
    370.0,
    IconName::Table,
    ShowDatabase,
    22
);

pub(super) fn init(cx: &mut App) {
    session::register_flush_on_quit(cx);
}

pub(super) fn bind_keys(cx: &mut App) {
    for context in ["ZedxDatabase", "ZedxDatabase > Editor && mode == full"] {
        cx.bind_keys([KeyBinding::new("ctrl-enter", ExecuteQuery, Some(context))]);
    }
}

impl Render for Database {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .key_context("ZedxDatabase")
            .on_action(
                cx.listener(|this, _: &ExecuteQuery, window, cx| this.execute(false, window, cx)),
            )
            .child(ui::zedx::tool_window_head(
                "database",
                ui::zedx::Icon::Database,
                "Database",
                cx,
            ))
            .child(
                h_flex()
                    .h(px(34.0))
                    .flex_none()
                    .gap_1()
                    .px_2()
                    .child(
                        Button::new("open-sqlite", "Open SQLite…").on_click(
                            cx.listener(|this, _, window, cx| this.choose_file(window, cx)),
                        ),
                    )
                    .child(
                        ui::IconButton::new("refresh-sqlite", IconName::RotateCw)
                            .tooltip(Tooltip::text("Refresh Tables"))
                            .disabled(self.path.is_none() || self.pending)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.execute(true, window, cx)),
                            ),
                    )
                    .child(
                        ui::IconButton::new("disconnect-sqlite", IconName::Close)
                            .tooltip(Tooltip::text("Disconnect"))
                            .disabled(self.path.is_none())
                            .on_click(cx.listener(|this, _, _, cx| this.disconnect(cx))),
                    ),
            )
            .child(self.render_session_controls(cx))
            .when_some(self.path.as_ref(), |view, path| {
                view.child(
                    div().px_2().py_1().child(
                        Label::new(path.display().to_string())
                            .size(ui::zedx::caption_size())
                            .color(Color::Muted)
                            .truncate(),
                    ),
                )
            })
            .when(self.path.is_none(), |view| {
                view.child(
                    div().p_3().child(
                        Label::new(
                            "Open a local SQLite file to browse tables and run read-only queries.",
                        )
                        .color(Color::Muted),
                    ),
                )
            })
            .child(
                v_flex()
                    .id("database-schema")
                    .max_h(px(180.0))
                    .overflow_y_scroll()
                    .flex_none()
                    .children(self.schema.iter().map(|name| {
                        let sql = inspect::table_query(name);
                        Button::new(
                            SharedString::from(format!("database-table-{name}")),
                            name.clone(),
                        )
                        .full_width()
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.query.update(cx, |query, cx| {
                                    query.set_text(sql.clone(), window, cx)
                                });
                                this.execute(false, window, cx);
                            },
                        ))
                    })),
            )
            .child(
                div()
                    .h(px(80.0))
                    .flex_none()
                    .p_2()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .child(self.query.clone()),
            )
            .child(
                h_flex()
                    .px_2()
                    .py_1()
                    .gap_2()
                    .child(
                        Button::new("execute-sqlite", "Run  Ctrl+Enter")
                            .disabled(self.path.is_none())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.execute(false, window, cx)),
                            ),
                    )
                    .when(self.pending, |row| {
                        row.child(Label::new("Running…").color(Color::Muted))
                    })
                    .child(
                        Button::new("cancel-sqlite-query", "Cancel")
                            .disabled(!self.pending)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel.store(true, Ordering::Relaxed);
                                cx.notify();
                            })),
                    ),
            )
            .when_some(self.error.clone(), |view, error| {
                view.child(div().p_2().child(Label::new(error).color(Color::Error)))
            })
            .child(
                div()
                    .id("database-results")
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        v_flex()
                            .w(px(self.results.columns.len() as f32 * 150.0))
                            .child(
                                h_flex().children(
                                    self.results
                                        .columns
                                        .iter()
                                        .map(|column| database_cell(column, true, cx)),
                                ),
                            )
                            .children(self.results.rows.iter().map(|row| {
                                h_flex().children(
                                    row.iter().map(|value| database_cell(value, false, cx)),
                                )
                            })),
                    ),
            )
            .child(
                div().px_2().py_1().child(
                    Label::new(format!(
                        "{} rows{} · Read only",
                        self.results.rows.len(),
                        if self.results.truncated {
                            " (first 256)"
                        } else {
                            ""
                        }
                    ))
                    .size(ui::zedx::caption_size())
                    .color(Color::Muted),
                ),
            )
    }
}
fn database_cell(text: &str, header: bool, cx: &App) -> impl IntoElement + use<> {
    div()
        .w(px(150.0))
        .h(px(26.0))
        .flex_none()
        .px_2()
        .border_b_1()
        .border_r_1()
        .border_color(cx.theme().colors().border_variant)
        .when(header, |cell| {
            cell.bg(cx.theme().colors().surface_background)
        })
        .child(
            Label::new(text.to_string())
                .size(ui::zedx::caption_size())
                .truncate(),
        )
}

#[cfg(test)]
#[path = "zedx_database_tests.rs"]
mod tests;

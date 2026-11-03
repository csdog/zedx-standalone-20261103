use super::*;
use db::kvp::KeyValueStore;
use serde::{Deserialize, Serialize};
use ui::{ContextMenu, IconButton, PopoverMenu};

const NAMESPACE: &str = "zedx_database_sessions";
const CONNECTION_LIMIT: usize = 12;
const QUERY_LIMIT: usize = 30;
const SQL_LIMIT: usize = 64 * 1024;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Saved {
    pub(super) connections: Vec<PathBuf>,
    pub(super) selected: Option<PathBuf>,
    pub(super) draft: String,
    history: Vec<Query>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Query {
    database: PathBuf,
    sql: String,
}
#[derive(Default)]
pub(super) struct Session {
    pub(super) saved: Saved,
    key: Option<String>,
    write: Option<Task<()>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(write) = self.write.take() {
            write.detach();
        }
    }
}
impl Saved {
    fn sanitize(mut self) -> Self {
        let mut seen = HashSet::new();
        self.connections
            .retain(|path| path.is_absolute() && seen.insert(path.clone()));
        self.connections.truncate(CONNECTION_LIMIT);
        if !self
            .selected
            .as_ref()
            .is_some_and(|path| self.connections.contains(path))
        {
            self.selected = None;
        }
        if self.draft.len() > SQL_LIMIT {
            self.draft.clear();
        }
        let mut queries = HashSet::new();
        self.history.retain(|query| {
            self.connections.contains(&query.database)
                && !query.sql.trim().is_empty()
                && query.sql.len() <= SQL_LIMIT
                && queries.insert((query.database.clone(), query.sql.clone()))
        });
        self.history.truncate(QUERY_LIMIT);
        self
    }
    fn remember_connection(&mut self, path: PathBuf) {
        self.connections.retain(|candidate| candidate != &path);
        self.connections.insert(0, path.clone());
        self.connections.truncate(CONNECTION_LIMIT);
        self.selected = Some(path);
        self.history
            .retain(|query| self.connections.contains(&query.database));
    }
    fn remember_query(&mut self, database: PathBuf, sql: String) {
        if sql.trim().is_empty() || sql.len() > SQL_LIMIT {
            return;
        }
        self.history
            .retain(|query| query.database != database || query.sql != sql);
        self.history.insert(0, Query { database, sql });
        self.history.truncate(QUERY_LIMIT);
    }
}
fn scope_key(roots: &[PathBuf]) -> anyhow::Result<Option<String>> {
    if roots.is_empty() {
        return Ok(None);
    }
    let mut roots = roots.to_vec();
    roots.sort();
    roots.dedup();
    Ok(Some(serde_json::to_string(&roots)?))
}
impl Session {
    pub(super) fn load(workspace: &Workspace, cx: &App) -> (Self, Option<String>) {
        let scope = if workspace.project().read(cx).is_local() {
            scope_key(
                &workspace
                    .root_paths(cx)
                    .iter()
                    .map(|path| path.to_path_buf())
                    .collect::<Vec<_>>(),
            )
        } else {
            Ok(None)
        };
        let key = match scope {
            Ok(key) => key,
            Err(error) => {
                return (
                    Self::default(),
                    Some(format!("Could not identify database session: {error}")),
                );
            }
        };
        let loaded: anyhow::Result<Saved> = (|| {
            let Some(key) = &key else {
                return Ok(Saved::default());
            };
            let json = KeyValueStore::global(cx).scoped(NAMESPACE).read(key)?;
            json.map(|json| serde_json::from_str(&json))
                .transpose()
                .map(|saved| saved.unwrap_or_default())
                .map_err(anyhow::Error::from)
        })();
        match loaded {
            Ok(saved) => (
                Self {
                    saved: saved.sanitize(),
                    key,
                    write: None,
                },
                None,
            ),
            Err(error) => (
                Self {
                    key,
                    saved: Saved::default(),
                    write: None,
                },
                Some(format!("Could not restore database session: {error}")),
            ),
        }
    }
}
pub(super) fn register_flush_on_quit(cx: &mut App) {
    cx.on_app_quit(|cx| {
        let mut tasks = Vec::new();
        for window in cx
            .windows()
            .into_iter()
            .filter_map(|window| window.downcast::<workspace::MultiWorkspace>())
        {
            window
                .update(cx, |multi_workspace, _, cx| {
                    for workspace in multi_workspace.workspaces() {
                        if let Some(panel) = workspace.read(cx).panel::<Database>(cx) {
                            if let Some(task) =
                                panel.update(cx, |panel, cx| panel.flush_session(cx))
                            {
                                tasks.push(task);
                            }
                        }
                    }
                })
                .log_err();
        }
        async move {
            futures::future::join_all(tasks).await;
        }
    })
    .detach();
}
impl Database {
    pub(super) fn flush_session(&mut self, cx: &mut Context<Self>) -> Option<Task<()>> {
        self.persist_session(cx);
        let previous = self.session.write.take()?;
        let (sender, receiver) = futures::channel::oneshot::channel();
        // Keep subsequent writes ordered even when a caller starts a flush.
        self.session.write = Some(cx.spawn(async move |_, _| {
            previous.await;
            if sender.send(()).is_err() {
                log::debug!("Database session flush caller was closed");
            }
        }));
        Some(cx.background_spawn(async move {
            receiver.await.log_err();
        }))
    }

    pub(super) fn persist_session(&mut self, cx: &mut Context<Self>) {
        let draft = self.query.read(cx).text(cx);
        if draft.len() > SQL_LIMIT {
            self.error = Some("The query draft exceeds 64 KiB and could not be saved.".into());
            cx.notify();
        } else {
            self.session.saved.draft = draft;
        }
        let Some(key) = self.session.key.clone() else {
            return;
        };
        let json = match serde_json::to_string(&self.session.saved) {
            Ok(json) => json,
            Err(error) => {
                self.error = Some(format!("Could not save database session: {error}"));
                cx.notify();
                return;
            }
        };
        let store = KeyValueStore::global(cx);
        let previous = self.session.write.take();
        self.session.write = Some(cx.spawn(async move |panel, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            if let Err(error) = store.scoped(NAMESPACE).write(key, json).await {
                panel
                    .update(cx, |panel, cx| {
                        panel.error = Some(format!("Could not save database session: {error}"));
                        cx.notify();
                    })
                    .log_err();
            }
        }));
    }
    pub(super) fn connected_session(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.path.clone() {
            self.session.saved.remember_connection(path);
        }
        self.persist_session(cx);
    }
    pub(super) fn completed_query(&mut self, sql: String, cx: &mut Context<Self>) {
        if let Some(path) = self.path.clone() {
            self.session.saved.remember_query(path, sql);
        }
        self.persist_session(cx);
    }
    fn select_connection(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.session.saved.connections.contains(&path) {
            self.error = Some("This connection was removed. Open the database file again.".into());
            cx.notify();
            return;
        }
        self.disconnect(cx);
        self.path = Some(path);
        self.execute(true, window, cx);
    }
    pub(super) fn forget_connection(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            return;
        };
        self.session
            .saved
            .connections
            .retain(|candidate| candidate != &path);
        self.session
            .saved
            .history
            .retain(|query| query.database != path);
        self.disconnect(cx);
    }
    fn fill_history_query(&mut self, query: &Query, window: &mut Window, cx: &mut Context<Self>) {
        if self.path.as_ref() != Some(&query.database) {
            self.error =
                Some("The database connection changed. Open its current query history.".into());
            cx.notify();
            return;
        }
        self.query.update(cx, |editor, cx| {
            editor.set_text(query.sql.clone(), window, cx)
        });
        window.focus(&self.query.focus_handle(cx), cx);
    }
    pub(super) fn render_session_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = cx.weak_entity();
        let connections = self.session.saved.connections.clone();
        let history = self
            .session
            .saved
            .history
            .iter()
            .filter(|query| Some(&query.database) == self.path.as_ref())
            .cloned()
            .collect::<Vec<_>>();
        let history_panel = panel.clone();
        let clear_panel = panel.clone();
        h_flex()
            .px_2()
            .py_1()
            .gap_1()
            .flex_none()
            .child(
                PopoverMenu::new("database-connections")
                    .trigger(
                        IconButton::new("database-connections-trigger", IconName::Server)
                            .tooltip(Tooltip::text("Saved Connections"))
                            .disabled(connections.is_empty()),
                    )
                    .menu(move |window, cx| {
                        let panel = panel.clone();
                        let connections = connections.clone();
                        Some(ContextMenu::build(window, cx, move |mut menu, _, _| {
                            for path in connections {
                                let panel = panel.clone();
                                menu = menu.entry(
                                    path.display().to_string(),
                                    None,
                                    move |window, cx| {
                                        panel
                                            .update(cx, |panel, cx| {
                                                panel.select_connection(path.clone(), window, cx)
                                            })
                                            .log_err();
                                    },
                                );
                            }
                            menu
                        }))
                    }),
            )
            .child(
                PopoverMenu::new("database-query-history")
                    .trigger(
                        IconButton::new("database-query-history-trigger", IconName::Clock)
                            .tooltip(Tooltip::text("Query History"))
                            .disabled(history.is_empty()),
                    )
                    .menu(move |window, cx| {
                        let panel = history_panel.clone();
                        let clear_panel = clear_panel.clone();
                        let history = history.clone();
                        Some(ContextMenu::build(window, cx, move |mut menu, _, _| {
                            for query in history {
                                let label = query
                                    .sql
                                    .split_whitespace()
                                    .collect::<Vec<_>>()
                                    .join(" ")
                                    .chars()
                                    .take(80)
                                    .collect::<String>();
                                let panel = panel.clone();
                                menu = menu.entry(label, None, move |window, cx| {
                                    panel
                                        .update(cx, |panel, cx| {
                                            panel.fill_history_query(&query, window, cx);
                                        })
                                        .log_err();
                                });
                            }
                            let panel = clear_panel.clone();
                            menu.separator().entry(
                                "Clear History for This Database",
                                None,
                                move |_, cx| {
                                    panel
                                        .update(cx, |panel, cx| {
                                            let selected = panel.path.clone();
                                            panel.session.saved.history.retain(|query| {
                                                Some(&query.database) != selected.as_ref()
                                            });
                                            panel.persist_session(cx);
                                            cx.notify();
                                        })
                                        .log_err();
                                },
                            )
                        }))
                    }),
            )
            .child(
                IconButton::new("database-forget-connection", IconName::Trash)
                    .disabled(self.path.is_none())
                    .tooltip(Tooltip::text(
                        "Remove connection and its query history; keep the database file",
                    ))
                    .on_click(cx.listener(|this, _, _, cx| this.forget_connection(cx))),
            )
    }
}

#[cfg(test)]
#[path = "zedx_database_session_tests.rs"]
mod tests;

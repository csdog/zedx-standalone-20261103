use crate::Workspace;
use collections::HashMap;
use gpui::{Action, App, Global, Task};
use project::{Project, TaskSourceKind};
use serde::{Deserialize, Serialize};
use task::{ResolvedTask, TaskTemplate};
use util::ResultExt;

const HISTORY_LIMIT: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentRunConfiguration {
    pub key: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, schemars::JsonSchema, Action)]
#[action(namespace = zedx)]
pub struct RunRecentConfiguration {
    pub key: String,
}

#[derive(Default)]
struct HistoryCache {
    entries: HashMap<String, Vec<RecentRunConfiguration>>,
    writes: HashMap<String, Task<()>>,
}
impl Global for HistoryCache {}

pub fn configuration_key(
    project: &Project,
    source: &TaskSourceKind,
    task: &TaskTemplate,
    cx: &App,
) -> Option<String> {
    let source = match source {
        TaskSourceKind::Worktree {
            id,
            directory_in_worktree,
            ..
        } => {
            let worktree = project.worktree_for_id(*id, cx)?;
            format!(
                "worktree:{}:{}",
                worktree.read(cx).abs_path().display(),
                directory_in_worktree.as_unix_str()
            )
        }
        TaskSourceKind::AbsPath { abs_path, .. } => format!("global:{}", abs_path.display()),
        TaskSourceKind::Language { name } => format!("language:{name}"),
        TaskSourceKind::UserInput | TaskSourceKind::Lsp { .. } => return None,
    };
    serde_json::to_string(&(source, &task.label)).log_err()
}

fn scope(workspace: &Workspace, cx: &App) -> Option<String> {
    let project = workspace.project().read(cx);
    if !crate::zedx_workbench::enabled() || project.is_via_collab() {
        return None;
    }
    let mut roots = project
        .visible_worktrees(cx)
        .map(|worktree| worktree.read(cx).abs_path().to_path_buf())
        .collect::<Vec<_>>();
    roots.sort();
    if roots.is_empty() {
        return None;
    }
    Some(format!(
        "zedx.recent-run-configurations.v1:{}",
        serde_json::to_string(&(project.remote_connection_options(cx), &roots)).log_err()?
    ))
}

fn load(scope: &str, cx: &App) -> Vec<RecentRunConfiguration> {
    if let Some(entries) = cx
        .try_global::<HistoryCache>()
        .and_then(|cache| cache.entries.get(scope))
    {
        return entries.clone();
    }
    db::kvp::KeyValueStore::global(cx)
        .read_kvp(scope)
        .log_err()
        .flatten()
        .and_then(|value| serde_json::from_str(&value).log_err())
        .unwrap_or_default()
}

fn promote(entries: &mut Vec<RecentRunConfiguration>, entry: RecentRunConfiguration) {
    entries.retain(|previous| previous.key != entry.key);
    entries.insert(0, entry);
    entries.truncate(HISTORY_LIMIT);
}

pub fn recent(workspace: &Workspace, cx: &App) -> Vec<RecentRunConfiguration> {
    scope(workspace, cx)
        .map(|scope| load(&scope, cx))
        .unwrap_or_default()
}

pub fn initialize(workspace: &Workspace, cx: &mut App) {
    let Some(scope) = scope(workspace, cx) else {
        return;
    };
    let entries = load(&scope, cx);
    cx.default_global::<HistoryCache>()
        .entries
        .entry(scope)
        .or_insert(entries);
}

pub(super) fn record(
    workspace: &Workspace,
    source: &TaskSourceKind,
    task: &ResolvedTask,
    cx: &mut App,
) {
    let Some(scope) = scope(workspace, cx) else {
        return;
    };
    let Some(key) = configuration_key(
        workspace.project().read(cx),
        source,
        task.original_task(),
        cx,
    ) else {
        return;
    };
    let mut entries = load(&scope, cx);
    promote(
        &mut entries,
        RecentRunConfiguration {
            key,
            label: task.resolved_label.clone(),
        },
    );
    let Some(value) = serde_json::to_string(&entries).log_err() else {
        return;
    };
    let previous = cx.default_global::<HistoryCache>().writes.remove(&scope);
    cx.default_global::<HistoryCache>()
        .entries
        .insert(scope.clone(), entries);
    let db = db::kvp::KeyValueStore::global(cx);
    let write_scope = scope.clone();
    let write = cx.spawn(async move |_| {
        // Serialize snapshots so a slower earlier write cannot overwrite a newer selection.
        if let Some(previous) = previous {
            previous.await;
        }
        db.write_kvp(write_scope, value).await.log_err();
    });
    cx.default_global::<HistoryCache>()
        .writes
        .insert(scope, write);
}

pub fn prioritize(workspace: &Workspace, tasks: &mut [(TaskSourceKind, ResolvedTask)], cx: &App) {
    let entries = recent(workspace, cx);
    let project = workspace.project().read(cx);
    tasks.sort_by_key(|(source, task)| {
        let key = configuration_key(project, source, task.original_task(), cx);
        entries
            .iter()
            .position(|entry| Some(&entry.key) == key.as_ref())
            .unwrap_or(usize::MAX)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_configurations_round_trip_deduplicate_and_move_to_front() {
        let mut entries = Vec::new();
        for index in 0..12 {
            promote(
                &mut entries,
                RecentRunConfiguration {
                    key: index.to_string(),
                    label: format!("Configuration {index}"),
                },
            );
        }
        promote(
            &mut entries,
            RecentRunConfiguration {
                key: "5".into(),
                label: "Renamed display label".into(),
            },
        );
        let restored: Vec<RecentRunConfiguration> =
            serde_json::from_str(&serde_json::to_string(&entries).unwrap()).unwrap();
        assert_eq!(restored, entries);
        assert_eq!(restored.len(), HISTORY_LIMIT);
        assert_eq!(restored[0].key, "5");
        assert_eq!(restored.iter().filter(|entry| entry.key == "5").count(), 1);
    }
    #[gpui::test]
    async fn history_survives_cache_reload_and_is_isolated_between_projects(
        cx: &mut gpui::TestAppContext,
    ) {
        crate::tests::init_test(cx);
        cx.update(|cx| cx.set_global(db::AppDatabase::test_new()));
        let fs = fs::FakeFs::new(cx.executor());
        fs.insert_tree("/first", serde_json::json!({"main.rs": "fn main() {}"}))
            .await;
        fs.insert_tree("/second", serde_json::json!({"main.rs": "fn main() {}"}))
            .await;
        let first = Project::test(fs.clone(), [std::path::Path::new("/first")], cx).await;
        let second = Project::test(fs, [std::path::Path::new("/second")], cx).await;
        let first = cx.add_window(|window, cx| Workspace::test_new(first, window, cx));
        let second = cx.add_window(|window, cx| Workspace::test_new(second, window, cx));
        let write = first
            .update(cx, |workspace, _, cx| {
                let source = TaskSourceKind::AbsPath {
                    id_base: "global".into(),
                    abs_path: "/global/tasks.json".into(),
                };
                for label in ["Build", "Run", "Build"] {
                    let template = TaskTemplate {
                        label: label.into(),
                        command: "true".into(),
                        ..Default::default()
                    };
                    let task = template
                        .resolve_task("global", &Default::default())
                        .unwrap();
                    record(workspace, &source, &task, cx);
                }
                let scope = scope(workspace, cx).unwrap();
                cx.default_global::<HistoryCache>()
                    .writes
                    .remove(&scope)
                    .unwrap()
            })
            .unwrap();
        write.await;
        cx.update(|cx| cx.default_global::<HistoryCache>().entries.clear());
        first
            .update(cx, |workspace, _, cx| {
                assert_eq!(
                    recent(workspace, cx)
                        .iter()
                        .map(|entry| entry.label.as_str())
                        .collect::<Vec<_>>(),
                    ["Build", "Run"]
                );
            })
            .unwrap();
        second
            .update(cx, |workspace, _, cx| {
                assert!(recent(workspace, cx).is_empty())
            })
            .unwrap();
    }
}

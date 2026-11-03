use super::*;
use gpui::TestAppContext;

#[test]
fn saved_session_prunes_invalid_connections_queries_and_duplicate_entries() {
    let database = PathBuf::from("/project/data.sqlite");
    let saved = Saved {
        connections: vec![database.clone(), database.clone(), "relative.sqlite".into()],
        selected: Some("/missing.sqlite".into()),
        draft: "x".repeat(SQL_LIMIT + 1),
        history: vec![
            Query {
                database: database.clone(),
                sql: "SELECT 1".into(),
            },
            Query {
                database: database.clone(),
                sql: "SELECT 1".into(),
            },
            Query {
                database: "/other.sqlite".into(),
                sql: "SELECT 2".into(),
            },
        ],
    }
    .sanitize();
    assert_eq!(saved.connections, vec![database]);
    assert!(saved.selected.is_none());
    assert!(saved.draft.is_empty());
    assert_eq!(saved.history.len(), 1);
}
#[test]
fn query_history_is_bounded_recency_ordered_and_isolated_per_database() {
    let mut saved = Saved::default();
    let first = PathBuf::from("/first.sqlite");
    let second = PathBuf::from("/second.sqlite");
    saved.remember_connection(first.clone());
    saved.remember_connection(second.clone());
    for number in 0..QUERY_LIMIT + 5 {
        saved.remember_query(first.clone(), format!("SELECT {number}"));
    }
    assert_eq!(saved.history.len(), QUERY_LIMIT);
    saved.remember_query(second.clone(), "SELECT 7".into());
    saved.remember_query(first.clone(), "SELECT 7".into());
    assert_eq!(saved.history[0].database, first);
    assert_eq!(saved.history[1].database, second);
    assert_eq!(
        saved
            .history
            .iter()
            .filter(|query| query.database == first && query.sql == "SELECT 7")
            .count(),
        1
    );
    assert_eq!(saved.history.len(), QUERY_LIMIT);
}
#[test]
fn older_empty_session_remains_compatible() {
    let saved: Saved = serde_json::from_str("{}").unwrap();
    assert!(saved.connections.is_empty());
    assert!(saved.history.is_empty());
    assert!(saved.selected.is_none());
}

#[gpui::test]
async fn reopening_panel_restores_connection_and_draft_without_executing_sql(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let panel = workspace.read_with(cx, |workspace, cx| workspace.panel::<Database>(cx).unwrap());
    let write = panel.update_in(cx, |panel, window, cx| {
        panel.path = Some(PathBuf::from("/project/missing.sqlite"));
        panel.connected_session(cx);
        panel.query.update(cx, |editor, cx| {
            editor.set_text("SELECT 'draft only'", window, cx)
        });
        panel.completed_query("SELECT 42".into(), cx);
        panel.session.write.take().unwrap()
    });
    write.await;
    let restored = workspace.update_in(cx, |workspace, window, cx| {
        cx.new(|cx| Database::new(workspace, window, cx))
    });
    restored.read_with(cx, |panel, cx| {
        assert_eq!(panel.path, Some(PathBuf::from("/project/missing.sqlite")));
        assert_eq!(panel.query.read(cx).text(cx), "SELECT 'draft only'");
        assert_eq!(panel.session.saved.history[0].sql, "SELECT 42");
        assert!(panel.restoring_session);
        assert!(!panel.pending);
        assert!(panel.results.rows.is_empty());
        assert!(panel.error.is_none());
    });
    cx.update(|_, cx| {
        let other = scope_key(&[PathBuf::from("/different-project")])
            .unwrap()
            .unwrap();
        assert!(
            KeyValueStore::global(cx)
                .scoped(NAMESPACE)
                .read(&other)
                .unwrap()
                .is_none()
        );
    });
}
#[gpui::test]
async fn disconnect_keeps_bookmarks_but_removal_forgets_only_selected_database(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let panel = workspace.read_with(cx, |workspace, cx| workspace.panel::<Database>(cx).unwrap());
    panel.update(cx, |panel, cx| {
        for path in ["/first.sqlite", "/second.sqlite"] {
            panel.path = Some(path.into());
            panel.connected_session(cx);
            panel.completed_query("SELECT 1".into(), cx);
        }
        panel.disconnect(cx);
        assert!(panel.session.saved.selected.is_none());
        assert_eq!(panel.session.saved.connections.len(), 2);
        panel.path = Some("/first.sqlite".into());
        panel.forget_connection(cx);
        assert_eq!(
            panel.session.saved.connections,
            vec![PathBuf::from("/second.sqlite")]
        );
        assert_eq!(panel.session.saved.history.len(), 1);
        assert_eq!(
            panel.session.saved.history[0].database,
            PathBuf::from("/second.sqlite")
        );
        assert!(panel.path.is_none());
    });
}

#[test]
fn project_scope_is_stable_for_root_order_and_keeps_other_projects_separate() {
    let first = scope_key(&["/first".into(), "/second".into()]).unwrap();
    assert_eq!(
        first,
        scope_key(&["/second".into(), "/first".into(), "/first".into()]).unwrap()
    );
    assert_ne!(first, scope_key(&["/different".into()]).unwrap());
    assert_eq!(scope_key(&[]).unwrap(), None);
}
#[gpui::test]
async fn history_from_previous_connection_does_not_replace_current_draft(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let panel = workspace.read_with(cx, |workspace, cx| workspace.panel::<Database>(cx).unwrap());
    panel.update_in(cx, |panel, window, cx| {
        panel.path = Some("/second.sqlite".into());
        panel.query.update(cx, |query, cx| {
            query.set_text("SELECT 'current draft'", window, cx)
        });
        panel.fill_history_query(
            &Query {
                database: "/first.sqlite".into(),
                sql: "SELECT 'wrong database'".into(),
            },
            window,
            cx,
        );
        assert_eq!(panel.query.read(cx).text(cx), "SELECT 'current draft'");
        assert!(panel.error.is_some());
        assert!(!panel.pending);
    });
}

#[gpui::test]
async fn session_flush_writes_the_last_draft_before_completion(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let panel = workspace.read_with(cx, |workspace, cx| workspace.panel::<Database>(cx).unwrap());
    let flush = panel.update_in(cx, |panel, window, cx| {
        panel.query.update(cx, |query, cx| {
            query.set_text("SELECT 'last edit'", window, cx)
        });
        panel.flush_session(cx).unwrap()
    });
    flush.await;
    workspace.read_with(cx, |workspace, cx| {
        let (saved, error) = Session::load(workspace, cx);
        assert!(error.is_none());
        assert_eq!(saved.saved.draft, "SELECT 'last edit'");
    });
}

#[test]
fn connection_list_keeps_recent_items_and_prunes_evicted_query_history() {
    let mut saved = Saved::default();
    for index in 0..CONNECTION_LIMIT + 3 {
        let path = PathBuf::from(format!("/database-{index}.sqlite"));
        saved.remember_connection(path.clone());
        saved.remember_query(path, "SELECT 1".into());
    }
    assert_eq!(saved.connections.len(), CONNECTION_LIMIT);
    assert_eq!(
        saved.connections[0],
        PathBuf::from(format!("/database-{}.sqlite", CONNECTION_LIMIT + 2))
    );
    assert!(
        saved
            .history
            .iter()
            .all(|query| saved.connections.contains(&query.database))
    );
    let existing = saved.connections[3].clone();
    saved.remember_connection(existing.clone());
    assert_eq!(saved.connections[0], existing);
    assert_eq!(saved.connections.len(), CONNECTION_LIMIT);
}

#[gpui::test]
async fn malformed_saved_session_reports_error_and_leaves_database_disconnected(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let (store, key) = workspace.read_with(cx, |workspace, cx| {
        (
            KeyValueStore::global(cx),
            scope_key(
                &workspace
                    .root_paths(cx)
                    .iter()
                    .map(|path| path.to_path_buf())
                    .collect::<Vec<_>>(),
            )
            .unwrap()
            .unwrap(),
        )
    });
    store
        .scoped(NAMESPACE)
        .write(key, "{invalid json".into())
        .await
        .unwrap();
    workspace.read_with(cx, |workspace, cx| {
        let (session, error) = Session::load(workspace, cx);
        assert!(error.is_some());
        assert!(session.saved.selected.is_none());
        assert!(session.saved.connections.is_empty());
        assert!(session.saved.history.is_empty());
    });
}

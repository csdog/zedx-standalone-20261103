use super::*;
use gpui::TestAppContext;

#[gpui::test]
async fn database_rail_opens_real_panel_and_disconnect_clears_results(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    cx.dispatch_action(ShowDatabase);
    cx.run_until_parked();
    let panel = workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.right_dock().read(cx);
        assert!(dock.is_open());
        assert_eq!(dock.active_panel().unwrap().persistent_name(), "Database");
        workspace.panel::<Database>(cx).unwrap()
    });
    panel.update(cx, |panel, cx| {
        panel.path = Some(PathBuf::from("/missing.sqlite"));
        panel.schema = vec!["table".into()];
        panel.results.rows = vec![vec!["old value".into()]];
        let previous = panel.cancel.clone();
        let generation = panel.generation;
        panel.disconnect(cx);
        assert!(previous.load(Ordering::Relaxed));
        assert!(panel.generation > generation);
        assert!(panel.path.is_none());
        assert!(panel.schema.is_empty());
        assert!(panel.results.rows.is_empty());
    });
}

#[gpui::test]
async fn missing_database_reports_error_without_creating_file(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let panel = workspace.read_with(cx, |workspace, cx| workspace.panel::<Database>(cx).unwrap());
    let path = std::env::temp_dir().join(format!("zedx-missing-{}.sqlite", uuid::Uuid::new_v4()));
    panel.update_in(cx, |panel, window, cx| {
        panel.path = Some(path.clone());
        panel.execute(true, window, cx);
    });
    cx.run_until_parked();
    panel.read_with(cx, |panel, _| {
        assert!(!panel.pending);
        assert!(panel.error.is_some());
        assert!(panel.schema.is_empty());
    });
    assert!(!path.exists());
}

#[gpui::test]
async fn ctrl_enter_executes_from_query_editor_without_inserting_newline(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
    let panel = workspace.read_with(cx, |workspace, cx| workspace.panel::<Database>(cx).unwrap());
    panel.update_in(cx, |panel, window, cx| {
        panel.path = Some(
            std::env::temp_dir().join(format!("zedx-shortcut-{}.sqlite", uuid::Uuid::new_v4())),
        );
        panel
            .query
            .update(cx, |query, cx| query.set_text("SELECT 1", window, cx));
    });
    cx.dispatch_action(ShowDatabase);
    cx.run_until_parked();
    cx.simulate_keystrokes("ctrl-enter");
    cx.run_until_parked();
    panel.read_with(cx, |panel, cx| {
        assert_eq!(panel.query.read(cx).text(cx), "SELECT 1");
        assert!(
            panel.error.is_some(),
            "the query reached the backend and reported the missing file"
        );
    });
}

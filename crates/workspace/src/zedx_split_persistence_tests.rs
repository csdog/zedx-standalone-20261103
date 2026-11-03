use super::*;
use fs::FakeFs;
use gpui::{Entity, TestAppContext, VisualTestContext};
use serde_json::json;
use std::path::Path;

async fn setup(cx: &mut TestAppContext, id: i64) -> (Entity<Workspace>, VisualTestContext) {
    crate::tests::init_test(cx);
    cx.update(|cx| cx.set_global(db::AppDatabase::test_new()));
    let fs = FakeFs::new(cx.executor());
    fs.insert_tree("/project", json!({"main.rs":"fn main() {}"}))
        .await;
    let project = project::Project::test(fs, [Path::new("/project")], cx).await;
    let window = cx.add_window(|window, cx| Workspace::test_new(project, window, cx));
    let workspace = window.update(cx, |_, _, cx| cx.entity()).unwrap();
    workspace.update(cx, |workspace, _| {
        workspace.set_database_id(crate::WorkspaceId::from_i64(id))
    });
    let visual = VisualTestContext::from_window(window.into(), cx);
    (workspace, visual)
}

#[test]
fn saved_split_accepts_older_data_and_sanitizes_fraction() {
    let default: SavedSplit = serde_json::from_str("{}").unwrap();
    assert!(!default.enabled);
    assert_eq!(default.fraction, None);
    let saved: SavedSplit = serde_json::from_str(r#"{"enabled":true,"fraction":12.0}"#).unwrap();
    assert_eq!(saved.sanitized().fraction, Some(0.95));
    assert_eq!(
        SavedSplit {
            enabled: true,
            fraction: Some(f32::NAN)
        }
        .sanitized()
        .fraction,
        None
    );
}

#[gpui::test]
async fn split_loads_per_workspace_and_retains_preferences_before_panels_arrive(
    cx: &mut TestAppContext,
) {
    let (workspace, mut cx) = setup(cx, 42).await;
    let store = cx.update(|_, cx| KeyValueStore::global(cx));
    store
        .scoped(NAMESPACE)
        .write("42".into(), r#"{"enabled":true,"fraction":0.72}"#.into())
        .await
        .unwrap();
    workspace.update_in(&mut cx, |workspace, window, cx| {
        let dock = workspace.left_dock().clone();
        dock.update(cx, |dock, cx| {
            dock.load_zedx_split(workspace, window, cx);
            assert!(dock.zedx_split.enabled);
            assert_eq!(dock.zedx_split.fraction, Some(0.72));
            dock.zedx_split.fraction = Some(0.65);
            dock.load_zedx_split(workspace, window, cx);
            assert_eq!(dock.zedx_split.fraction, Some(0.65));
        });
        workspace.set_database_id(crate::WorkspaceId::from_i64(43));
        dock.update(cx, |dock, cx| {
            dock.zedx_split = Default::default();
            dock.load_zedx_split(workspace, window, cx);
            assert!(!dock.zedx_split.enabled);
        });
    });
}

#[gpui::test]
async fn reset_is_persisted_after_previous_split_writes(cx: &mut TestAppContext) {
    let (workspace, mut cx) = setup(cx, 42).await;
    let write = workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.left_dock().update(cx, |dock, cx| {
            dock.load_zedx_split(workspace, window, cx);
            dock.zedx_split.enabled = true;
            dock.zedx_split.fraction = Some(0.7);
            dock.persist_zedx_split(cx);
            dock.zedx_split.fraction = Some(0.8);
            dock.persist_zedx_split(cx);
            dock.reset_zedx_split(window, cx);
            dock.zedx_split.persistence.write.take().unwrap()
        })
    });
    write.await;
    cx.update(|_, cx| {
        let json = KeyValueStore::global(cx)
            .scoped(NAMESPACE)
            .read("42")
            .unwrap()
            .unwrap();
        let saved: SavedSplit = serde_json::from_str(&json).unwrap();
        assert!(!saved.enabled);
        assert_eq!(saved.fraction, None);
    });
}

#[gpui::test]
async fn serialization_flush_waits_for_latest_split_layout(cx: &mut TestAppContext) {
    let (workspace, mut cx) = setup(cx, 59).await;
    let flush = workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.left_dock().update(cx, |dock, cx| {
            dock.load_zedx_split(workspace, window, cx);
            dock.zedx_split.enabled = true;
            dock.zedx_split.fraction = Some(0.77);
            dock.zedx_split.resizing = true;
        });
        workspace.flush_serialization(window, cx)
    });
    flush.await;
    cx.update(|_, cx| {
        let value = KeyValueStore::global(cx)
            .scoped(NAMESPACE)
            .read("59")
            .unwrap()
            .unwrap();
        let saved: SavedSplit = serde_json::from_str(&value).unwrap();
        assert!(saved.enabled);
        assert_eq!(saved.fraction, Some(0.77));
    });
}
#[gpui::test]
async fn flush_barrier_keeps_later_layout_writes_ordered(cx: &mut TestAppContext) {
    let (workspace, mut cx) = setup(cx, 60).await;
    let (first, last) = workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.left_dock().update(cx, |dock, cx| {
            dock.load_zedx_split(workspace, window, cx);
            dock.zedx_split.enabled = true;
            dock.zedx_split.fraction = Some(0.65);
            dock.persist_zedx_split(cx);
            let first = dock.flush_zedx_split(cx).unwrap();
            dock.zedx_split.fraction = Some(0.85);
            dock.persist_zedx_split(cx);
            let last = dock.flush_zedx_split(cx).unwrap();
            (first, last)
        })
    });
    futures::join!(first, last);
    cx.update(|_, cx| {
        let value = KeyValueStore::global(cx)
            .scoped(NAMESPACE)
            .read("60")
            .unwrap()
            .unwrap();
        let saved: SavedSplit = serde_json::from_str(&value).unwrap();
        assert_eq!(saved.fraction, Some(0.85));
    });
}

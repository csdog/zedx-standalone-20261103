use super::*;
use gpui::TestAppContext;

#[test]
fn notification_age_handles_boundaries_and_clock_changes() {
    assert_eq!(notification_age(60, 61), "Just now");
    assert_eq!(notification_age(59, 0), "Just now");
    assert_eq!(notification_age(60, 0), "1 min ago");
    assert_eq!(notification_age(3600, 0), "1 h ago");
    assert_eq!(notification_age(86400, 0), "1 d ago");
}

#[gpui::test]
async fn center_keeps_dismissed_toasts_and_deduplicates_updates(cx: &mut TestAppContext) {
    crate::tests::init_test(cx);
    let fs = fs::FakeFs::new(cx.executor());
    fs.insert_tree("/project", serde_json::json!({"main.rs": ""}))
        .await;
    let project = project::Project::test(fs, [std::path::Path::new("/project")], cx).await;
    let window = cx.add_window(|window, cx| Workspace::test_new(project, window, cx));
    window
        .update(cx, |workspace, _, cx| {
            for message in ["first", "updated"] {
                workspace.show_toast(
                    Toast::new(NotificationId::named("saved".into()), message),
                    cx,
                );
            }
            assert_eq!(workspace.notifications.center.entries.len(), 1);
            assert_eq!(workspace.notifications.center.unread_count(), 1);
            workspace.dismiss_notification(&NotificationId::named("saved".into()), cx);
            assert!(workspace.notifications.is_empty());
            assert_eq!(workspace.notifications.center.entries.len(), 1);
            workspace.toggle_zedx_notification_center(cx);
            assert!(workspace.notifications.center.open);
            assert_eq!(workspace.notifications.center.unread_count(), 0);
            workspace.remove_zedx_notification(&NotificationId::named("saved".into()), cx);
            assert!(workspace.notifications.center.entries.is_empty());
        })
        .unwrap();
}

#[gpui::test]
async fn notification_archive_is_bounded_without_losing_newest(cx: &mut TestAppContext) {
    crate::tests::init_test(cx);
    let fs = fs::FakeFs::new(cx.executor());
    fs.insert_tree("/project", serde_json::json!({})).await;
    let project = project::Project::test(fs, [std::path::Path::new("/project")], cx).await;
    let window = cx.add_window(|window, cx| Workspace::test_new(project, window, cx));
    window
        .update(cx, |workspace, _, cx| {
            for index in 0..105 {
                workspace.show_toast(
                    Toast::new(
                        NotificationId::named(format!("event-{index}").into()),
                        "event",
                    ),
                    cx,
                );
            }
            let state = &workspace.notifications.center;
            assert_eq!(state.entries.len(), 100);
            assert_eq!(
                state.entries[0].id,
                NotificationId::named("event-104".into())
            );
            assert_eq!(
                state.entries[99].id,
                NotificationId::named("event-5".into())
            );
        })
        .unwrap();
}

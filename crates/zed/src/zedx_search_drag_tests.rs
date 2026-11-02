use super::*;
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, size};

fn drag_handle(cx: &mut VisualTestContext, delta: gpui::Point<gpui::Pixels>) {
    let start = cx.debug_bounds("zedx-search-drag-handle").unwrap().center();
    let end = start + delta;
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
}

#[gpui::test]
async fn zedx_search_header_drag_moves_without_dismissing_or_losing_input(cx: &mut TestAppContext) {
    let (workspace, cx) = tests::setup(cx).await;
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    cx.run_until_parked();
    cx.refresh().unwrap();
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    let before = cx.debug_bounds("zedx-search-modal").unwrap();
    drag_handle(cx, point(px(80.0), px(60.0)));
    let after = cx.debug_bounds("zedx-search-modal").unwrap();
    assert_eq!(after.origin, before.origin + point(px(80.0), px(60.0)));
    assert_eq!(
        workspace
            .read_with(cx, |workspace, cx| workspace
                .active_modal::<SearchWindow>(cx))
            .unwrap()
            .entity_id(),
        modal.entity_id()
    );
    modal.update_in(cx, |modal, window, cx| {
        assert!(modal.focus_handle(cx).contains_focused(window, cx))
    });
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query("main", window, cx))
    });
    cx.run_until_parked();
    assert!(modal.read_with(cx, |modal, cx| {
        modal
            .picker
            .read(cx)
            .delegate
            .rows
            .iter()
            .any(|row| matches!(row.target, Target::File(_)) && row.label == "main.rs")
    }));
    cx.dispatch_action(menu::Cancel);
    cx.run_until_parked();
    assert!(
        workspace
            .read_with(cx, |workspace, cx| workspace
                .active_modal::<SearchWindow>(cx))
            .is_none()
    );
}

#[gpui::test]
async fn zedx_search_drag_clamps_and_refits_when_viewport_changes(cx: &mut TestAppContext) {
    let (workspace, cx) = tests::setup(cx).await;
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    workspace.update_in(cx, |workspace, window, cx| open(workspace, window, cx));
    cx.run_until_parked();
    cx.refresh().unwrap();
    drag_handle(cx, point(px(2000.0), px(2000.0)));
    for viewport in [
        size(px(1280.0), px(900.0)),
        size(px(390.0), px(500.0)),
        size(px(700.0), px(600.0)),
    ] {
        cx.simulate_resize(viewport);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let bounds = cx.debug_bounds("zedx-search-modal").unwrap();
        assert!(
            bounds.left() >= px(0.0) && bounds.right() <= viewport.width,
            "{bounds:?} in {viewport:?}"
        );
        assert!(
            bounds.top() >= px(0.0) && bounds.bottom() <= viewport.height,
            "{bounds:?} in {viewport:?}"
        );
        let footer = cx.debug_bounds("zedx-search-footer").unwrap();
        assert!(footer.bottom() <= bounds.bottom());
    }
    drag_handle(cx, point(px(-2000.0), px(-2000.0)));
    let bounds = cx.debug_bounds("zedx-search-modal").unwrap();
    assert_eq!(bounds.origin, point(px(8.0), px(8.0)));
}

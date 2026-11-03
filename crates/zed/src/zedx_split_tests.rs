use super::*;
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, size};
use project_panel::ProjectPanel;

async fn setup(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    cx.update(project_panel::init);
    let (workspace, cx) = super::tests::setup(cx).await;
    let project = workspace
        .update_in(cx, |_, window, cx| {
            cx.spawn_in(window, async move |workspace, cx| {
                ProjectPanel::load(workspace, cx.clone()).await
            })
        })
        .await
        .unwrap();
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.add_panel(project, window, cx);
        workspace.focus_panel::<ProjectPanel>(window, cx);
    });
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    (workspace, cx)
}

fn drag(cx: &mut VisualTestContext, source: &'static str, position: gpui::Point<gpui::Pixels>) {
    let start = cx.debug_bounds(source).unwrap().center();
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(position, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
}

#[gpui::test]
async fn structure_rail_drop_splits_project_and_each_hide_keeps_the_other(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    let host = cx.debug_bounds("zedx-left-split-host").unwrap();
    let target = point(host.center().x, host.top() + host.size.height * 0.75);
    drag(cx, "zedx-tool-Structure", target);
    let project = cx
        .debug_bounds("zedx-split-project")
        .expect("Project remains above Structure");
    let structure = cx.debug_bounds("zedx-split-structure").unwrap();
    assert!(project.bottom() <= structure.top());
    assert_eq!(project.left(), structure.left());
    assert_eq!(project.size.width, structure.size.width);
    assert!(
        cx.debug_bounds("zedx-selected-tool-Project Panel")
            .is_some()
    );
    assert!(cx.debug_bounds("zedx-selected-tool-Structure").is_some());
    workspace.update_in(cx, |workspace, window, cx| {
        let focus = workspace.active_pane().read(cx).focus_handle(cx);
        window.focus(&focus, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-split-structure").is_some());
    let hide = cx.debug_bounds("zedx-structure-hide").unwrap();
    cx.simulate_click(hide.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-split-structure").is_none());
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.left_dock().read(cx);
        assert!(dock.is_open());
        assert_eq!(
            dock.active_panel().unwrap().persistent_name(),
            "Project Panel"
        );
    });
    drag(cx, "zedx-tool-Structure", target);
    let project_hide = cx.debug_bounds("zedx-project-hide").unwrap();
    cx.simulate_click(project_hide.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    workspace.read_with(cx, |workspace, cx| {
        let dock = workspace.left_dock().read(cx);
        assert!(dock.is_open());
        assert_eq!(dock.active_panel().unwrap().persistent_name(), "Structure");
    });
}

#[gpui::test]
async fn structure_header_drop_resize_compact_and_reset(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    cx.dispatch_action(ToggleStructure);
    cx.run_until_parked();
    cx.refresh().unwrap();
    let host = cx.debug_bounds("zedx-left-split-host").unwrap();
    drag(
        cx,
        "zedx-head-structure",
        point(host.center().x, host.top() + host.size.height * 0.8),
    );
    let before = cx.debug_bounds("zedx-split-project").unwrap();
    let handle = cx.debug_bounds("zedx-left-split-resize").unwrap();
    drag(
        cx,
        "zedx-left-split-resize",
        handle.center() + point(px(0.0), px(70.0)),
    );
    let after = cx.debug_bounds("zedx-split-project").unwrap();
    assert!((after.size.height - before.size.height - px(70.0)).abs() < px(3.0));
    let start = cx.debug_bounds("zedx-left-split-resize").unwrap().center();
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        start + point(px(0.0), px(-30.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.run_until_parked();
    cx.refresh().unwrap();
    let end = start + point(px(0.0), px(-60.0));
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    let continuous = cx.debug_bounds("zedx-split-project").unwrap();
    assert!((continuous.size.height - after.size.height + px(60.0)).abs() < px(3.0));
    for viewport in [size(px(390.0), px(500.0)), size(px(1280.0), px(900.0))] {
        cx.simulate_resize(viewport);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let host = cx.debug_bounds("zedx-left-split-host").unwrap();
        let project = cx.debug_bounds("zedx-split-project").unwrap();
        let structure = cx.debug_bounds("zedx-split-structure").unwrap();
        assert!(project.top() >= host.top());
        assert!(structure.bottom() <= host.bottom() + px(1.0));
        assert!(structure.size.height > px(30.0));
    }
    cx.dispatch_action(workspace::zedx_actions::RestoreDefaultLayout);
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-split-structure").is_none());
    workspace.read_with(cx, |workspace, cx| {
        assert!(workspace.left_dock().read(cx).is_open())
    });
}

#[gpui::test]
async fn invalid_drops_are_inert_and_switching_tools_preserves_the_split(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    let host = cx.debug_bounds("zedx-left-split-host").unwrap();
    drag(
        cx,
        "zedx-tool-Structure",
        point(host.center().x, host.top() + px(20.0)),
    );
    assert!(cx.debug_bounds("zedx-split-structure").is_none());
    drag(
        cx,
        "zedx-tool-Structure",
        point(host.right() + px(150.0), host.center().y),
    );
    assert!(cx.debug_bounds("zedx-split-structure").is_none());
    let target = point(host.center().x, host.top() + host.size.height * 0.8);
    drag(cx, "zedx-tool-Run", target);
    assert!(cx.debug_bounds("zedx-split-structure").is_none());
    drag(cx, "zedx-tool-Structure", target);
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<Find>(window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-split-structure").is_some());
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<Branches>(window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-split-structure").is_none());
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.focus_panel::<ProjectPanel>(window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-split-structure").is_some());
    let structure_pin = cx.debug_bounds("zedx-tool-Structure").unwrap();
    cx.simulate_click(structure_pin.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-split-structure").is_none());
    assert!(cx.debug_bounds("zedx-project-hide").is_some());
}

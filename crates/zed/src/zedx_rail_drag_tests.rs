use super::*;
use git_ui::git_panel::GitPanel;
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point};
use project_panel::ProjectPanel;

const LEFT: [&str; 3] = [
    "zedx-tool-Project Panel",
    "zedx-tool-GitPanel",
    "zedx-tool-Structure",
];
const BOTTOM: [&str; 3] = [
    "zedx-tool-Git history",
    "zedx-tool-Run",
    "zedx-tool-Problems",
];

async fn setup(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    cx.update(project_panel::init);
    let (workspace, cx) = super::tests::setup(cx).await;
    cx.update(|_, cx| language_model::init(cx));
    workspace
        .update_in(cx, |_, window, cx| {
            cx.spawn_in(window, async move |workspace, cx| {
                let project = ProjectPanel::load(workspace.clone(), cx.clone()).await?;
                let git = GitPanel::load(workspace.clone(), cx.clone()).await?;
                workspace.update_in(cx, |workspace, window, cx| {
                    workspace.add_panel(project, window, cx);
                    workspace.add_panel(git, window, cx);
                    git_ui::git_panel::register(workspace);
                })?;
                anyhow::Ok(())
            })
        })
        .await
        .unwrap();
    cx.run_until_parked();
    cx.refresh().unwrap();
    (workspace, cx)
}

fn order(cx: &mut VisualTestContext, names: &[&'static str]) -> Vec<&'static str> {
    let mut names = names.to_vec();
    names.sort_by(|left, right| {
        cx.debug_bounds(left)
            .unwrap()
            .top()
            .partial_cmp(&cx.debug_bounds(right).unwrap().top())
            .unwrap()
    });
    names
}

fn drag(cx: &mut VisualTestContext, start: gpui::Point<Pixels>, end: gpui::Point<Pixels>) {
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        point(start.x + px(12.0), start.y),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
}

#[gpui::test]
async fn zedx_rail_drag_outside_buttons_does_not_reorder(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let start = cx.debug_bounds(LEFT[2]).unwrap().center();
    let project = cx.debug_bounds(LEFT[0]).unwrap();
    drag(cx, start, point(px(300.0), project.top() + px(2.0)));
    assert_eq!(order(cx, &LEFT), LEFT);
    assert!(cx.debug_bounds("zedx-tool-Find").is_none());
    assert!(cx.debug_bounds("zedx-tool-Branches").is_none());
}

#[gpui::test]
async fn zedx_rail_drag_boundaries_keep_hidden_tools_unpinned(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let project = cx.debug_bounds(LEFT[0]).unwrap();
    let commit = cx.debug_bounds(LEFT[1]).unwrap();
    drag(
        cx,
        project.center(),
        point(commit.center().x, commit.top() + px(2.0)),
    );
    assert_eq!(order(cx, &LEFT), LEFT);
    let project = cx.debug_bounds(LEFT[0]).unwrap();
    let commit = cx.debug_bounds(LEFT[1]).unwrap();
    drag(
        cx,
        project.center(),
        point(commit.center().x, commit.bottom() - px(2.0)),
    );
    assert_eq!(order(cx, &LEFT), [LEFT[1], LEFT[0], LEFT[2]]);
    assert!(cx.debug_bounds("zedx-tool-Find").is_none());
    assert!(cx.debug_bounds("zedx-tool-Branches").is_none());
    let structure = cx.debug_bounds(LEFT[2]).unwrap();
    let commit = cx.debug_bounds(LEFT[1]).unwrap();
    drag(
        cx,
        structure.center(),
        point(commit.center().x, commit.top() + px(2.0)),
    );
    assert_eq!(order(cx, &LEFT), [LEFT[2], LEFT[1], LEFT[0]]);
    cx.dispatch_action(workspace::zedx_actions::RestoreDefaultLayout);
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(order(cx, &LEFT), LEFT);
}

#[gpui::test]
async fn zedx_rail_drag_between_docks_does_not_change_either_order(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let start = cx.debug_bounds(BOTTOM[1]).unwrap().center();
    let project = cx.debug_bounds(LEFT[0]).unwrap();
    drag(
        cx,
        start,
        point(project.center().x, project.top() + px(2.0)),
    );
    assert_eq!(order(cx, &LEFT), LEFT);
    assert_eq!(order(cx, &BOTTOM), BOTTOM);
    assert!(cx.debug_bounds("zedx-tool-Find").is_none());
    assert!(cx.debug_bounds("zedx-tool-Branches").is_none());
}

#[gpui::test]
async fn zedx_rail_drag_bottom_group_reorders_without_exposing_more_tools(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let problems = cx.debug_bounds(BOTTOM[2]).unwrap();
    let git = cx.debug_bounds(BOTTOM[0]).unwrap();
    drag(
        cx,
        problems.center(),
        point(git.center().x, git.top() + px(2.0)),
    );
    assert_eq!(order(cx, &BOTTOM), [BOTTOM[2], BOTTOM[0], BOTTOM[1]]);
    let git = cx.debug_bounds(BOTTOM[0]).unwrap();
    let run = cx.debug_bounds(BOTTOM[1]).unwrap();
    drag(
        cx,
        git.center(),
        point(run.center().x, run.bottom() - px(2.0)),
    );
    assert_eq!(order(cx, &BOTTOM), [BOTTOM[2], BOTTOM[1], BOTTOM[0]]);
    assert_eq!(order(cx, &LEFT), LEFT);
    assert!(cx.debug_bounds("zedx-tool-DebugPanel").is_none());
    cx.dispatch_action(workspace::zedx_actions::RestoreDefaultLayout);
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(order(cx, &BOTTOM), BOTTOM);
}

#[gpui::test]
async fn zedx_rail_drag_right_outside_buttons_does_not_reorder(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let right = [
        "zedx-tool-AgentPanel",
        "zedx-tool-Outline Panel",
        "zedx-database",
    ];
    let database = cx.debug_bounds(right[2]).unwrap();
    let assistant = cx.debug_bounds(right[0]).unwrap();
    drag(
        cx,
        database.center(),
        point(px(300.0), assistant.top() + px(2.0)),
    );
    assert_eq!(order(cx, &right), right);
    assert_eq!(order(cx, &LEFT), LEFT);
    assert_eq!(order(cx, &BOTTOM), BOTTOM);
}

#[gpui::test]
async fn zedx_rail_drag_single_move_then_release_reorders(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let start = cx.debug_bounds(LEFT[2]).unwrap().center();
    let project = cx.debug_bounds(LEFT[0]).unwrap();
    let end = point(project.center().x, project.top() + px(2.0));
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(order(cx, &LEFT), [LEFT[2], LEFT[0], LEFT[1]]);
}

#[gpui::test]
async fn zedx_rail_selection_tracks_visible_panels_independently_of_focus(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace
            .dock_at_position(DockPosition::Left)
            .update(cx, |dock, cx| dock.set_open(false, window, cx));
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    for (target, selected, other) in [
        (
            LEFT[0],
            "zedx-selected-tool-Project Panel",
            "zedx-selected-tool-GitPanel",
        ),
        (
            LEFT[1],
            "zedx-selected-tool-GitPanel",
            "zedx-selected-tool-Project Panel",
        ),
    ] {
        let center = cx.debug_bounds(target).unwrap().center();
        cx.simulate_click(center, Modifiers::none());
        cx.run_until_parked();
        cx.refresh().unwrap();
        let highlight = cx
            .debug_bounds(selected)
            .expect("active tool must paint a selected background");
        assert_eq!(highlight.size, gpui::size(px(32.0), px(34.0)));
        assert!(cx.debug_bounds(other).is_none());
    }
    let bottom = cx.debug_bounds(BOTTOM[2]).unwrap().center();
    cx.simulate_click(bottom, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-selected-tool-GitPanel").is_some());
    assert!(cx.debug_bounds("zedx-selected-tool-Problems").is_some());
    workspace.update_in(cx, |workspace, window, cx| {
        window.focus(&workspace.focus_handle(cx), cx)
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-selected-tool-GitPanel").is_some());
    let center = cx.debug_bounds(LEFT[1]).unwrap().center();
    cx.simulate_click(center, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-selected-tool-GitPanel").is_none());
    assert!(cx.debug_bounds("zedx-selected-tool-Problems").is_some());
}

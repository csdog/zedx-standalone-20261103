use super::*;
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, size};
use workspace::zedx_actions::{
    MoveToolWindow, RestoreDefaultLayout, ToggleRightAssistant, ToggleRightOutline,
};

struct AssistantFixture {
    focus: FocusHandle,
    zoomed: bool,
}
impl AssistantFixture {
    fn activation_focus(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
    fn on_active(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) {}
    fn badge(&self, _: &App) -> Option<String> {
        None
    }
}
fixed_panel!(
    AssistantFixture,
    "AgentPanel",
    "RightSidebarAssistantFixture",
    DockPosition::Right,
    270.0,
    IconName::Sparkle,
    ToggleRightAssistant,
    20
);
impl Render for AssistantFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus)
            .child(Label::new("Assistant fixture without a provider"))
    }
}

async fn setup(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    let (workspace, cx) = super::tests::setup(cx).await;
    let outline = workspace
        .update_in(cx, |_, window, cx| {
            cx.spawn_in(window, async move |workspace, cx| {
                OutlinePanel::load(workspace, cx.clone()).await
            })
        })
        .await
        .unwrap();
    workspace.update_in(cx, |workspace, window, cx| {
        workspace.add_panel(outline, window, cx);
        let assistant = cx.new(|cx| AssistantFixture {
            focus: cx.focus_handle(),
            zoomed: false,
        });
        workspace.add_panel(assistant, window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    (workspace, cx)
}

#[gpui::test]
async fn right_controls_stay_in_the_upper_right_and_toggle_panels(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    for viewport in [size(px(1280.0), px(900.0)), size(px(390.0), px(500.0))] {
        cx.simulate_resize(viewport);
        cx.refresh().unwrap();
        let rail = cx.debug_bounds("zedx-right-rail").unwrap();
        let controls = [
            "zedx-notifications-button",
            "zedx-tool-AgentPanel",
            "zedx-tool-Outline Panel",
            "zedx-database",
        ]
        .map(|name| cx.debug_bounds(name).expect(name));
        for bounds in controls {
            assert!(
                rail.contains(&bounds.center()),
                "every pinned control belongs to the right rail"
            );
        }
        for pair in controls.windows(2) {
            assert_eq!(pair[1].top() - pair[0].bottom(), px(4.0));
        }
        assert!(controls[3].bottom() < rail.center().y);
    }
    let assistant = cx.debug_bounds("zedx-tool-AgentPanel").unwrap();
    cx.simulate_click(assistant.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-head-assistant").is_some());
    workspace.read_with(cx, |workspace, cx| {
        let right = workspace.dock_at_position(DockPosition::Right).read(cx);
        assert!(right.is_open());
        assert_eq!(
            right.active_panel().unwrap().persistent_name(),
            "AgentPanel"
        );
    });
    let outline = cx.debug_bounds("zedx-tool-Outline Panel").unwrap();
    cx.simulate_click(outline.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-head-right-outline").is_some());
    workspace.read_with(cx, |workspace, cx| {
        let right = workspace.dock_at_position(DockPosition::Right).read(cx);
        assert_eq!(
            right.active_panel().unwrap().persistent_name(),
            "Outline Panel"
        );
    });
    let hide = cx.debug_bounds("zedx-right-outline-hide").unwrap();
    cx.simulate_click(hide.center(), Modifiers::none());
    cx.run_until_parked();
    workspace.update_in(cx, |workspace, window, cx| {
        assert!(
            !workspace
                .dock_at_position(DockPosition::Right)
                .read(cx)
                .is_open()
        );
        assert!(
            workspace
                .active_pane()
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx)
        );
    });
    cx.dispatch_action(ToggleRightAssistant);
    cx.dispatch_action(ToggleRightAssistant);
    assert!(!workspace.read_with(cx, |workspace, cx| {
        workspace
            .dock_at_position(DockPosition::Right)
            .read(cx)
            .is_open()
    }));
}

#[gpui::test]
async fn outline_button_waits_for_an_asynchronous_move_from_the_left(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    cx.update(|_, cx| {
        cx.update_global::<settings::SettingsStore, _>(|store, cx| {
            store
                .set_user_settings(r#"{"outline_panel":{"dock":"left"}}"#, cx)
                .unwrap();
        })
    });
    cx.run_until_parked();
    assert!(workspace.read_with(cx, |workspace, cx| {
        workspace
            .dock_at_position(DockPosition::Left)
            .read(cx)
            .panel_index_for_persistent_name("Outline Panel", cx)
            .is_some()
    }));
    cx.dispatch_action(ToggleRightOutline);
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert!(
            workspace
                .dock_at_position(DockPosition::Left)
                .read(cx)
                .panel_index_for_persistent_name("Outline Panel", cx)
                .is_none()
        );
        let right = workspace.dock_at_position(DockPosition::Right).read(cx);
        assert!(right.is_open());
        assert_eq!(
            right.active_panel().unwrap().persistent_name(),
            "Outline Panel"
        );
    });
}

#[gpui::test]
async fn unavailable_assistant_keeps_its_place_without_opening_an_empty_dock(
    cx: &mut TestAppContext,
) {
    let (workspace, cx) = super::tests::setup(cx).await;
    cx.refresh().unwrap();
    let assistant = cx
        .debug_bounds("zedx-tool-AgentPanel")
        .expect("disabled assistant pin remains visible");
    cx.simulate_click(assistant.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(!workspace.read_with(cx, |workspace, cx| {
        workspace
            .dock_at_position(DockPosition::Right)
            .read(cx)
            .is_open()
    }));
}

#[gpui::test]
async fn dragging_right_controls_reorders_and_reset_restores_them(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let start = cx.debug_bounds("zedx-database").unwrap().center();
    let before = cx.debug_bounds("zedx-tool-AgentPanel").unwrap();
    let end = point(before.center().x, before.top() + px(2.0));
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        point(start.x, start.y - px(12.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-database").unwrap().top()
            < cx.debug_bounds("zedx-tool-AgentPanel").unwrap().top()
    );
    cx.dispatch_action(MoveToolWindow {
        name: "Outline Panel".into(),
        delta: -2,
    });
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-tool-Outline Panel").unwrap().top()
            < cx.debug_bounds("zedx-database").unwrap().top()
    );
    cx.dispatch_action(RestoreDefaultLayout);
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-tool-AgentPanel").unwrap().top()
            < cx.debug_bounds("zedx-tool-Outline Panel").unwrap().top()
    );
    assert!(
        cx.debug_bounds("zedx-tool-Outline Panel").unwrap().top()
            < cx.debug_bounds("zedx-database").unwrap().top()
    );
}

#[gpui::test]
async fn right_rail_single_move_then_release_reorders(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx).await;
    let start = cx.debug_bounds("zedx-database").unwrap().center();
    let assistant = cx.debug_bounds("zedx-tool-AgentPanel").unwrap();
    let end = point(assistant.center().x, assistant.top() + px(2.0));
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-database").unwrap().top()
            < cx.debug_bounds("zedx-tool-AgentPanel").unwrap().top()
    );
}

#[gpui::test]
async fn right_selection_tracks_open_panel_and_survives_editor_focus(cx: &mut TestAppContext) {
    let (workspace, cx) = setup(cx).await;
    workspace.update_in(cx, |workspace, window, cx| {
        workspace
            .dock_at_position(DockPosition::Right)
            .update(cx, |dock, cx| dock.set_open(false, window, cx));
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-selected-right-AgentPanel").is_none());
    assert!(
        cx.debug_bounds("zedx-selected-right-Outline Panel")
            .is_none()
    );
    for (button, selected, previous) in [
        (
            "zedx-tool-AgentPanel",
            "zedx-selected-right-AgentPanel",
            "zedx-selected-right-Outline Panel",
        ),
        (
            "zedx-tool-Outline Panel",
            "zedx-selected-right-Outline Panel",
            "zedx-selected-right-AgentPanel",
        ),
    ] {
        let button = cx.debug_bounds(button).unwrap();
        cx.simulate_click(button.center(), Modifiers::none());
        cx.run_until_parked();
        cx.refresh().unwrap();
        let highlight = cx
            .debug_bounds(selected)
            .expect("active right panel paints its selection");
        assert_eq!(highlight.size, size(px(32.0), px(34.0)));
        assert_eq!(highlight, button);
        assert!(cx.debug_bounds(previous).is_none());
        workspace.update_in(cx, |workspace, window, cx| {
            let focus = workspace.active_pane().read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(
            cx.debug_bounds(selected).is_some(),
            "selection follows the visible panel, not focus"
        );
    }
    let button = cx.debug_bounds("zedx-tool-Outline Panel").unwrap();
    cx.simulate_click(button.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(
        cx.debug_bounds("zedx-selected-right-Outline Panel")
            .is_none()
    );
    cx.dispatch_action(ToggleRightAssistant);
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-selected-right-AgentPanel").is_some());
    let hide = cx.debug_bounds("zedx-assistant-hide").unwrap();
    cx.simulate_click(hide.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-selected-right-AgentPanel").is_none());
}

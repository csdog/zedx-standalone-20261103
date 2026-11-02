fn drag_zedx_commit_message(cx: &mut VisualTestContext, end: gpui::Point<Pixels>) {
    let start = cx
        .debug_bounds("zedx-commit-message-resize")
        .unwrap()
        .center();
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
}

#[gpui::test]
async fn test_zedx_commit_message_resize_drags_clamps_and_retains_draft(cx: &mut TestAppContext) {
    let (workspace, panel, mut cx) = setup_zedx_source_diff(cx).await;
    cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.open_panel::<GitPanel>(window, cx)
    });
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.commit_editor.update(cx, |editor, cx| {
            editor.set_text("Keep this draft\nAnother line", window, cx)
        })
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let initial = cx.debug_bounds("zedx-commit-message").unwrap();
    let divider = cx.debug_bounds("zedx-commit-message-resize").unwrap();
    assert_eq!(divider.size.height, px(5.0));
    drag_zedx_commit_message(&mut cx, divider.center() - gpui::point(px(0.0), px(80.0)));
    let resized = cx.debug_bounds("zedx-commit-message").unwrap();
    assert_eq!(resized.size.height, initial.size.height + px(80.0));
    assert!(resized.top() < initial.top());
    let stash = cx.debug_bounds("zedx-commit-tab-Stash").unwrap();
    cx.simulate_click(stash.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    let commit = cx.debug_bounds("zedx-commit-tab-Commit").unwrap();
    cx.simulate_click(commit.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(
        cx.debug_bounds("zedx-commit-message").unwrap().size.height,
        resized.size.height
    );
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.close_panel::<GitPanel>(window, cx);
        workspace.open_panel::<GitPanel>(window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(
        cx.debug_bounds("zedx-commit-message").unwrap().size.height,
        resized.size.height
    );
    drag_zedx_commit_message(&mut cx, gpui::point(px(-1000.0), px(-1000.0)));
    let maximum = cx.debug_bounds("zedx-commit-message").unwrap();
    let whole = cx.debug_bounds("zedx-commit-panel").unwrap();
    assert_eq!(maximum.size.height, whole.size.height - px(160.0));
    for viewport in [
        gpui::size(px(390.0), px(500.0)),
        gpui::size(px(1280.0), px(900.0)),
    ] {
        cx.simulate_resize(viewport);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let whole = cx.debug_bounds("zedx-commit-panel").unwrap();
        let message = cx.debug_bounds("zedx-commit-message").unwrap();
        let actions = cx.debug_bounds("zedx-commit-message-actions").unwrap();
        assert!(message.top() >= whole.top() + px(160.0));
        assert!(message.bottom() <= whole.bottom());
        assert!(actions.top() >= message.top() && actions.bottom() <= message.bottom());
    }
    drag_zedx_commit_message(&mut cx, gpui::point(px(4000.0), px(4000.0)));
    assert_eq!(
        cx.debug_bounds("zedx-commit-message").unwrap().size.height,
        px(170.0)
    );
    panel.read_with(&cx, |panel, cx| {
        assert_eq!(
            panel.commit_editor.read(cx).text(cx),
            "Keep this draft\nAnother line"
        );
        assert!(matches!(
            panel.commit_editor.read(cx).mode(),
            EditorMode::Full { .. }
        ));
    });
}

#[gpui::test]
async fn test_zedx_commit_message_resize_keyboard_and_double_click_reset(cx: &mut TestAppContext) {
    let (workspace, _, mut cx) = setup_zedx_source_diff(cx).await;
    cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.open_panel::<GitPanel>(window, cx)
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let initial = cx.debug_bounds("zedx-commit-message").unwrap().size.height;
    let divider = cx.debug_bounds("zedx-commit-message-resize").unwrap();
    cx.simulate_click(divider.center(), Modifiers::none());
    cx.run_until_parked();
    cx.simulate_keystrokes("up");
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(
        cx.debug_bounds("zedx-commit-message").unwrap().size.height,
        initial + px(16.0)
    );
    cx.simulate_keystrokes("down home");
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(
        cx.debug_bounds("zedx-commit-message").unwrap().size.height,
        initial
    );
    let divider = cx.debug_bounds("zedx-commit-message-resize").unwrap();
    drag_zedx_commit_message(&mut cx, divider.center() - gpui::point(px(0.0), px(50.0)));
    let position = cx
        .debug_bounds("zedx-commit-message-resize")
        .unwrap()
        .center();
    cx.simulate_event(gpui::MouseDownEvent {
        position,
        button: gpui::MouseButton::Left,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(gpui::MouseUpEvent {
        position,
        button: gpui::MouseButton::Left,
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert_eq!(
        cx.debug_bounds("zedx-commit-message").unwrap().size.height,
        initial
    );
}

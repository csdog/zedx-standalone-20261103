use super::*;
use editor::{SelectionEffects, scroll::Autoscroll};
use workspace::{Toast, notifications::NotificationId};

gpui::actions!(zedx, [ToggleSourceDiff]);

pub fn bind_keys(cx: &mut App) {
    if std::env::var_os("ZEDX_UPSTREAM_UI").is_none() {
        cx.bind_keys([gpui::KeyBinding::new(
            "ctrl-d",
            ToggleSourceDiff,
            Some("Workspace > Pane > Editor && mode == full"),
        )]);
    }
}

pub(crate) fn init(cx: &mut App) {
    if std::env::var_os("ZEDX_UPSTREAM_UI").is_some() {
        return;
    }
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(toggle_source_diff);
    })
    .detach();
}

fn cursor(editor: &Entity<Editor>, cx: &App) -> Option<Anchor> {
    let editor = editor.read(cx);
    editor
        .buffer()
        .read(cx)
        .text_anchor_for_position(editor.selections.newest_anchor().head(), cx)
        .map(|(_, anchor)| anchor)
}

fn restore_cursor(
    editor: &Entity<Editor>,
    cursor: Option<Anchor>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(cursor) = cursor else {
        return;
    };
    editor.update(cx, |editor, cx| {
        let snapshot = editor.buffer().read(cx).snapshot(cx);
        // A source location outside the collapsed diff excerpts has no visible counterpart.
        let Some(cursor) = snapshot.anchor_in_buffer(cursor) else {
            return;
        };
        editor.change_selections(
            SelectionEffects::scroll(Autoscroll::fit()).unfold(),
            window,
            cx,
            |selections| selections.select_ranges([cursor..cursor]),
        );
    });
}

fn toggle_source_diff(
    workspace: &mut Workspace,
    _: &ToggleSourceDiff,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    if let Some(view) = workspace.active_item_as::<SoloDiffView>(cx) {
        let view = view.read(cx);
        let buffer = view.buffer.clone();
        if buffer
            .read(cx)
            .file()
            .is_some_and(|file| file.disk_state().is_deleted())
        {
            workspace.show_toast(
                Toast::new(
                    NotificationId::unique::<ToggleSourceDiff>(),
                    "This file has been deleted; there is no source file to open.",
                )
                .autohide(),
                cx,
            );
            return;
        }
        // Source is the working version, so use the right-hand cursor even from the old side.
        let position = cursor(view.editor.read(cx).rhs_editor(), cx);
        let editor = workspace
            .open_project_item::<Editor>(None, buffer, true, true, true, false, window, cx);
        restore_cursor(&editor, position, window, cx);
        return;
    }
    let Some(editor) = workspace.active_item_as::<Editor>(cx) else {
        return;
    };
    let Some(buffer) = editor.read(cx).active_buffer(cx) else {
        return;
    };
    let position = cursor(&editor, cx);
    let existing = workspace
        .items_of_type::<SoloDiffView>(cx)
        .find(|view| view.read(cx).buffer == buffer);
    if let Some(view) = existing {
        workspace.activate_item(&view, true, true, window, cx);
        let editor = view.read(cx).editor.read(cx).rhs_editor().clone();
        restore_cursor(&editor, position, window, cx);
        editor.focus_handle(cx).focus(window, cx);
        return;
    }
    let Some((entry, repository)) = crate::file_diff_entry(workspace, cx) else {
        workspace.show_toast(
            Toast::new(
                NotificationId::unique::<ToggleSourceDiff>(),
                "There are no Git changes to show for this file.",
            )
            .autohide(),
            cx,
        );
        return;
    };
    let workspace_handle = workspace.weak_handle();
    // SoloDiffView reads its workspace while opening; let the action's update finish first.
    window.defer(cx, move |window, cx| {
        let open =
            SoloDiffView::open_or_focus(entry, repository, workspace_handle.clone(), window, cx);
        window
            .spawn(cx, async move |cx| {
                let view = open.await?;
                view.update_in(cx, |view, window, cx| {
                    let editor = view.editor.read(cx).rhs_editor().clone();
                    restore_cursor(&editor, position, window, cx);
                    editor.focus_handle(cx).focus(window, cx);
                })?;
                anyhow::Ok(())
            })
            .detach_and_notify_err(workspace_handle, window, cx);
    });
}

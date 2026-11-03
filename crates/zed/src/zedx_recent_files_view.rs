use super::*;
use git::status::FileStatus;

fn navigation() -> Vec<(&'static str, ui::zedx::Icon, &'static str, Box<dyn Action>)> {
    use workspace::zedx_actions::*;
    vec![
        (
            "Project",
            ui::zedx::Icon::Project,
            "⌘1",
            zed_actions::project_panel::ToggleFocus.boxed_clone(),
        ),
        (
            "Commit",
            ui::zedx::Icon::Commit,
            "⌘0",
            ToggleCommit.boxed_clone(),
        ),
        (
            "Structure",
            ui::zedx::Icon::Structure,
            "⌘7",
            ToggleStructure.boxed_clone(),
        ),
        (
            "Find in Files",
            ui::zedx::Icon::FindTool,
            "⌘⇧F",
            FindInFiles::default().boxed_clone(),
        ),
        ("Run", ui::zedx::Icon::Play, "⌘4", ToggleRun.boxed_clone()),
        (
            "Terminal",
            ui::zedx::Icon::Terminal,
            "⌥F12",
            terminal_view::terminal_panel::ToggleFocus.boxed_clone(),
        ),
        (
            "Problems",
            ui::zedx::Icon::Warning,
            "⌘6",
            ToggleProblems.boxed_clone(),
        ),
        (
            "Git",
            ui::zedx::Icon::Git,
            "⌘9",
            ShowGitHistory.boxed_clone(),
        ),
        (
            "Settings",
            ui::zedx::Icon::Settings,
            "",
            zed_actions::OpenSettings.boxed_clone(),
        ),
    ]
}

impl RecentWindow {
    fn navigate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, _, _, action)) = navigation().into_iter().nth(index) else {
            return;
        };
        let workspace = self.picker.read(cx).delegate.workspace.clone();
        cx.emit(DismissEvent);
        // Dismiss the modal before focusing and dispatching into the workspace.
        cx.spawn_in(window, async move |_, cx| {
            workspace
                .update_in(cx, |workspace, window, cx| {
                    window.focus(&workspace.focus_handle(cx), cx);
                    window.dispatch_action(action, cx);
                })
                .log_err();
        })
        .detach();
    }
}

impl Render for RecentWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let font = theme_settings::setup_ui_font(window, cx);
        let viewport = window.viewport_size();
        let width = px(840.0).min((viewport.width - px(24.0)).max(px(1.0)));
        let navigation_width = if width < px(600.0) {
            px(118.0)
        } else {
            px(184.0)
        };
        let top = (viewport.height * 0.12).min(px(90.0));
        let available = (viewport.height - top - px(12.0)).max(px(1.0));
        let count = self.picker.read(cx).delegate.rows.len();
        let body_height = (available - px(36.0)).max(px(1.0)).min(px(410.0));
        self.picker.update(cx, |picker, _| {
            picker.fit_zedx_search(
                width - navigation_width - px(2.0),
                body_height,
                px((count as f32 * 32.0).clamp(230.0, 330.0) + 75.0),
                window,
            )
        });
        let navigation_focused = self.navigation_focus.contains_focused(window, cx);
        let modal = v_flex()
            .id("zedx-recent-files-modal")
            .debug_selector(|| "zedx-recent-files-modal".into())
            .font(font)
            .key_context("ZedXRecentFiles")
            .absolute()
            .left((viewport.width - width) / 2.0)
            .top(top)
            .w(width)
            .max_h(available)
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().elevated_surface_background)
            .shadow_lg()
            .overflow_hidden()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_action(cx.listener(|this, _: &RecentFiles, window, cx| this.cycle(window, cx)))
            .on_action(cx.listener(|this, _: &FocusRecentTools, window, cx| {
                window.focus(&this.navigation_focus, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusRecentFileList, window, cx| {
                this.picker.focus_handle(cx).focus(window, cx);
                cx.notify();
            }))
            .child(
                h_flex()
                    .h(px(36.0))
                    .flex_none()
                    .px_3()
                    .gap_2()
                    .child(ui::zedx::icon(ui::zedx::Icon::History, cx).size(px(16.0)))
                    .child(
                        Label::new("Recent Files")
                            .size(ui::zedx::label_size())
                            .weight(gpui::FontWeight::MEDIUM),
                    )
                    .child(div().flex_1())
                    .child(
                        Label::new("Control+E")
                            .size(ui::zedx::caption_size())
                            .color(Color::Muted),
                    )
                    .child(
                        ui::zedx::tool_button(
                            "recent-files-close",
                            ui::zedx::Icon::Close,
                            "Close Recent Files",
                            cx,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.picker
                                .update(cx, |picker, cx| picker.cancel(&menu::Cancel, window, cx))
                        })),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .min_h_0()
                    .max_h(body_height)
                    .items_stretch()
                    .child(
                        v_flex()
                            .id("zedx-recent-quick-navigation")
                            .debug_selector(|| "zedx-recent-quick-navigation".into())
                            .key_context("ZedXRecentTools")
                            .track_focus(&self.navigation_focus)
                            .w(navigation_width)
                            .flex_none()
                            .min_h_0()
                            .overflow_y_scroll()
                            .border_r_1()
                            .border_color(cx.theme().colors().border)
                            .py_1()
                            .on_action(cx.listener(|this, _: &menu::SelectPrevious, _, cx| {
                                this.navigation_selected =
                                    (this.navigation_selected + navigation().len() - 1)
                                        % navigation().len();
                                cx.notify();
                            }))
                            .on_action(cx.listener(|this, _: &menu::SelectNext, _, cx| {
                                this.navigation_selected =
                                    (this.navigation_selected + 1) % navigation().len();
                                cx.notify();
                            }))
                            .on_action(cx.listener(|this, _: &menu::Confirm, window, cx| {
                                this.navigate(this.navigation_selected, window, cx)
                            }))
                            .on_action(cx.listener(|this, _: &menu::Cancel, window, cx| {
                                this.picker.update(cx, |picker, cx| {
                                    picker.cancel(&menu::Cancel, window, cx)
                                })
                            }))
                            .child(
                                div().px_3().py_1().child(
                                    Label::new("Quick Navigation")
                                        .size(ui::zedx::caption_size())
                                        .color(Color::Muted),
                                ),
                            )
                            .children(navigation().into_iter().enumerate().map(
                                |(index, (label, icon, shortcut, _))| {
                                    h_flex()
                                        .id(("recent-navigation", index))
                                        .debug_selector(move || {
                                            format!("zedx-recent-navigation-{index}")
                                        })
                                        .w_full()
                                        .h(gpui::rems(2.5))
                                        .flex_none()
                                        .px_2()
                                        .gap_2()
                                        .cursor_pointer()
                                        .when(
                                            navigation_focused && self.navigation_selected == index,
                                            |row| row.bg(cx.theme().colors().element_selected),
                                        )
                                        .hover(|row| row.bg(cx.theme().colors().element_hover))
                                        .child(ui::zedx::icon(icon, cx).size(px(16.0)))
                                        .child(
                                            div().flex_1().min_w_0().overflow_hidden().child(
                                                Label::new(label)
                                                    .size(ui::zedx::label_size())
                                                    .truncate(),
                                            ),
                                        )
                                        .when(width >= px(600.0), |row| {
                                            row.child(
                                                Label::new(shortcut)
                                                    .size(ui::zedx::caption_size())
                                                    .color(Color::Muted),
                                            )
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.navigate(index, window, cx)
                                        }))
                                },
                            )),
                    )
                    .child(
                        div()
                            .debug_selector(|| "zedx-recent-file-list".into())
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(self.picker.clone()),
                    ),
            );
        gpui::deferred(
            div()
                .absolute()
                .inset_0()
                .size_full()
                .occlude()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_, _, _, cx| cx.emit(DismissEvent)),
                )
                .child(modal),
        )
        .with_priority(2)
    }
}

fn status_hint(status: Option<FileStatus>, dirty: bool) -> (Color, &'static str, &'static str) {
    match status {
        Some(status) if status.is_conflicted() => (Color::Conflict, "!", "Merge conflict"),
        Some(status) if status.is_untracked() => (Color::Deleted, "?", "Unversioned file"),
        Some(status) if status.is_deleted() => (Color::Deleted, "D", "Deleted file"),
        Some(status) if status.is_modified() => (Color::Modified, "M", "Modified file"),
        Some(status) if status.is_created() => (Color::Created, "A", "Added file"),
        Some(status) if status.is_ignored() => (Color::Ignored, "I", "Ignored file"),
        _ if dirty => (Color::Modified, "", "Unsaved changes"),
        _ => (Color::Default, "", ""),
    }
}

impl RecentDelegate {
    pub(super) fn render_recent_file_row(
        &self,
        index: usize,
        row: &Row,
        selected: bool,
        cx: &mut Context<Picker<Self>>,
    ) -> AnyElement {
        let path = match &row.target {
            Target::File(path) => Some(path),
            _ => None,
        };
        let project = self.project.read(cx);
        let status = path.and_then(|path| {
            project
                .git_store()
                .read(cx)
                .project_path_git_status(path, cx)
        });
        let dirty = path.is_some_and(|path| self.dirty_paths.contains(path));
        let (color, marker, description) = status_hint(status, dirty);
        let color = if color == Color::Modified {
            Color::Custom(
                gpui::rgb(if cx.theme().appearance().is_light() {
                    0x0058d0
                } else {
                    0x6897bb
                })
                .into(),
            )
        } else {
            color
        };
        let tooltip = format!(
            "{} / {}{}{}{}",
            row.source,
            row.detail,
            if description.is_empty() { "" } else { "\n" },
            description,
            if dirty && description != "Unsaved changes" {
                " · Unsaved changes"
            } else {
                ""
            }
        );
        div()
            .debug_selector(move || format!("zedx-recent-file-row-{index}"))
            .child(
                ListItem::new(("recent-file", index))
                    .height(gpui::rems(2.25))
                    .toggle_state(selected)
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .gap_2()
                            .child(match &row.icon {
                                Some(icon) => ui::zedx::asset(icon.clone())
                                    .size(px(16.0))
                                    .into_any_element(),
                                None => ui::zedx::icon(ui::zedx::Icon::Code, cx)
                                    .size(px(16.0))
                                    .into_any_element(),
                            })
                            .child(
                                div().flex_1().min_w_0().overflow_hidden().child(
                                    HighlightedLabel::new(
                                        row.label.clone(),
                                        highlight_indices(&row.label, &self.query),
                                    )
                                    .size(ui::zedx::label_size())
                                    .color(color)
                                    .truncate(),
                                ),
                            )
                            .when(dirty, |row| {
                                row.child(
                                    div()
                                        .size(px(5.0))
                                        .flex_none()
                                        .rounded_full()
                                        .bg(Color::Warning.color(cx)),
                                )
                            })
                            .child(
                                div().w(px(16.0)).flex_none().child(
                                    Label::new(marker)
                                        .size(ui::zedx::caption_size())
                                        .color(color),
                                ),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .max_w(relative(0.38))
                                    .overflow_hidden()
                                    .child(
                                        Label::new(format!("{} / {}", row.source, row.detail))
                                            .size(ui::zedx::caption_size())
                                            .color(Color::Muted)
                                            .truncate_start(),
                                    ),
                            ),
                    )
                    .tooltip(Tooltip::text(tooltip)),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_status_hints_distinguish_git_state_and_unsaved_changes() {
        use git::status::StatusCode;
        assert_eq!(
            status_hint(Some(FileStatus::Untracked), false),
            (Color::Deleted, "?", "Unversioned file")
        );
        assert_eq!(
            status_hint(Some(FileStatus::worktree(StatusCode::Modified)), false).0,
            Color::Modified
        );
        assert_eq!(
            status_hint(Some(FileStatus::index(StatusCode::Added)), false).0,
            Color::Created
        );
        assert_eq!(status_hint(None, true).0, Color::Modified);
        assert_eq!(status_hint(None, false).0, Color::Default);
    }
}

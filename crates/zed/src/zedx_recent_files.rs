use super::*;
use ui::HighlightedLabel;
use workspace::notifications::NotifyTaskExt;

gpui::actions!(zedx, [RecentFiles, FocusRecentTools, FocusRecentFileList]);

pub(super) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-e", RecentFiles, Some("Workspace")),
        KeyBinding::new("ctrl-e", RecentFiles, Some("Workspace > Editor")),
        KeyBinding::new("ctrl-e", RecentFiles, Some("Workspace > Terminal")),
        KeyBinding::new("ctrl-e", RecentFiles, Some("ZedXRecentFiles")),
        KeyBinding::new("left", FocusRecentTools, Some("ZedXRecentFiles > Editor")),
        KeyBinding::new("right", FocusRecentFileList, Some("ZedXRecentTools")),
        KeyBinding::new("up", menu::SelectPrevious, Some("ZedXRecentTools")),
        KeyBinding::new("down", menu::SelectNext, Some("ZedXRecentTools")),
        KeyBinding::new("enter", menu::Confirm, Some("ZedXRecentTools")),
        KeyBinding::new("escape", menu::Cancel, Some("ZedXRecentTools")),
        KeyBinding::new("ctrl-e", RecentFiles, Some("ZedXRecentFiles > Editor")),
        KeyBinding::new("escape", menu::Cancel, Some("ZedXRecentFiles > Editor")),
        KeyBinding::new("enter", menu::Confirm, Some("ZedXRecentFiles > Editor")),
        KeyBinding::new(
            "shift-enter",
            menu::SecondaryConfirm,
            Some("ZedXRecentFiles > Editor"),
        ),
        KeyBinding::new("up", menu::SelectPrevious, Some("ZedXRecentFiles > Editor")),
        KeyBinding::new("down", menu::SelectNext, Some("ZedXRecentFiles > Editor")),
    ]);
}

pub(super) fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace
            .register_action(|workspace, _: &RecentFiles, window, cx| open(workspace, window, cx));
    })
    .detach();
}

fn open(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(modal) = workspace.active_modal::<RecentWindow>(cx) {
        modal.update(cx, |modal, cx| modal.cycle(window, cx));
        return;
    }
    let project = workspace.project().read(cx);
    let current = workspace
        .active_item(cx)
        .and_then(|item| item.project_path(cx));
    let dirty_paths = workspace
        .panes()
        .iter()
        .flat_map(|pane| {
            pane.read(cx)
                .items()
                .filter(|item| item.is_dirty(cx))
                .filter_map(|item| item.project_path(cx))
                .collect::<Vec<_>>()
        })
        .collect();
    let project_handle = workspace.project().clone();
    let entries = workspace
        .recent_navigation_history(Some(100), cx)
        .into_iter()
        .map(|(path, _)| path)
        .filter(|path| project.absolute_path(path, cx).is_some())
        .map(|path| SearchDelegate::file_row(path, project, cx))
        .collect();
    let workspace_handle = cx.weak_entity();
    workspace.toggle_modal(window, cx, move |window, cx| {
        let delegate = RecentDelegate {
            workspace: workspace_handle,
            project: project_handle.clone(),
            dirty_paths,
            entries,
            rows: Vec::new(),
            query: String::new(),
            current,
            selected: 0,
        };
        let picker = cx.new(|cx| {
            Picker::uniform_list(delegate, window, cx)
                .embedded()
                .initial_width(rems(42.0))
                .max_height(rems(25.0))
                .show_scrollbar(true)
        });
        cx.subscribe(&picker, |_, _, _: &DismissEvent, cx| cx.emit(DismissEvent))
            .detach();
        cx.observe(&picker, |_, _, cx| cx.notify()).detach();
        let git_store = project_handle.read(cx).git_store().clone();
        let git_observer = cx.observe(&git_store, |_, _, cx| cx.notify());
        RecentWindow {
            picker,
            navigation_focus: cx.focus_handle(),
            navigation_selected: 0,
            _git_observer: git_observer,
        }
    });
}

struct RecentWindow {
    picker: Entity<Picker<RecentDelegate>>,
    navigation_focus: FocusHandle,
    navigation_selected: usize,
    _git_observer: gpui::Subscription,
}

impl RecentWindow {
    fn cycle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.picker.update(cx, |picker, cx| {
            picker.select_next(&menu::SelectNext, window, cx)
        });
        self.picker.focus_handle(cx).focus(window, cx);
    }
}

impl ModalView for RecentWindow {
    fn render_bare(&self) -> bool {
        true
    }
}
impl EventEmitter<DismissEvent> for RecentWindow {}
impl Focusable for RecentWindow {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}
#[path = "zedx_recent_files_view.rs"]
mod view;

struct RecentDelegate {
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    dirty_paths: collections::HashSet<ProjectPath>,
    entries: Vec<Row>,
    rows: Vec<Row>,
    current: Option<ProjectPath>,
    query: String,
    selected: usize,
}

impl PickerDelegate for RecentDelegate {
    type ListItem = AnyElement;
    fn name() -> &'static str {
        "ZedX Recent Files"
    }
    fn placeholder_text(&self, _: &mut Window, _: &mut App) -> Arc<str> {
        "Search recent files…".into()
    }
    fn no_matches_text(&self, _: &mut Window, _: &mut App) -> Option<SharedString> {
        Some(
            if self.entries.is_empty() {
                "No recent files yet"
            } else {
                "No matching recent files"
            }
            .into(),
        )
    }
    fn match_count(&self) -> usize {
        self.rows.len()
    }
    fn selected_index(&self) -> usize {
        self.selected
    }
    fn set_selected_index(&mut self, index: usize, _: &mut Window, _: &mut Context<Picker<Self>>) {
        self.selected = index;
    }
    fn dismissed(&mut self, _: &mut Window, _: &mut Context<Picker<Self>>) {}
    fn update_matches(
        &mut self,
        query: String,
        _: &mut Window,
        _: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        self.query = query.trim().to_string();
        self.rows = self
            .entries
            .iter()
            .filter(|row| {
                self.query.is_empty()
                    || !highlight_indices(&row.label, &self.query).is_empty()
                    || !highlight_indices(&format!("{}/{}", row.source, row.detail), &self.query)
                        .is_empty()
            })
            .cloned()
            .collect();
        self.selected = if self.query.is_empty() {
            self.rows.iter().position(|row| matches!(&row.target, Target::File(path) if Some(path) != self.current.as_ref())).unwrap_or(0)
        } else {
            0
        };
        Task::ready(())
    }
    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(Target::File(path)) = self.rows.get(self.selected).map(|row| row.target.clone())
        else {
            return;
        };
        cx.emit(DismissEvent);
        self.workspace
            .update(cx, |workspace, cx| {
                let pane = secondary.then(|| workspace.adjacent_pane(window, cx).downgrade());
                workspace
                    .open_path(path, pane, true, window, cx)
                    .detach_and_notify_err(self.workspace.clone(), window, cx);
            })
            .log_err();
    }
    fn render_match(
        &self,
        index: usize,
        selected: bool,
        _: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<AnyElement> {
        let row = self.rows.get(index)?;
        Some(self.render_recent_file_row(index, row, selected, cx))
    }

    fn render_footer(&self, _: &mut Window, cx: &mut Context<Picker<Self>>) -> Option<AnyElement> {
        Some(
            h_flex()
                .h(px(28.0))
                .px(px(12.0))
                .gap(px(8.0))
                .border_t_1()
                .border_color(cx.theme().colors().border)
                .child(
                    Label::new(format!("{} recent files", self.rows.len()))
                        .size(ui::zedx::caption_size())
                        .color(Color::Muted),
                )
                .child(div().flex_1())
                .child(
                    Label::new("Enter to open")
                        .size(ui::zedx::caption_size())
                        .color(Color::Muted),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
#[path = "zedx_recent_files_tests.rs"]
mod tests;

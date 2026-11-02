use editor::Editor;
use gpui::{Action, App, Context, Entity, Focusable, Render, Subscription, WeakEntity, Window};
use project::git_store::{GitStore, GitStoreEvent, Repository};
use ui::{Tooltip, prelude::*};
use util::ResultExt;
use workspace::{
    HideStatusItem, ItemHandle, StatusItemView, Toast, Workspace, notifications::NotificationId,
};

pub(crate) struct ProjectState {
    workspace: WeakEntity<Workspace>,
    git: Entity<GitStore>,
    repository: Option<Entity<Repository>>,
    repository_observer: Option<Subscription>,
    editor_observer: Option<Subscription>,
    _observers: Vec<Subscription>,
}

impl ProjectState {
    pub(crate) fn new(workspace: &Workspace, cx: &mut Context<Self>) -> Self {
        let git = workspace.project().read(cx).git_store().clone();
        let handle = workspace.weak_handle();
        let mut observers = vec![cx.subscribe(&git, |this, _, event, cx| {
            if matches!(
                event,
                GitStoreEvent::ActiveRepositoryChanged(_)
                    | GitStoreEvent::RepositoryAdded
                    | GitStoreEvent::RepositoryRemoved(_)
                    | GitStoreEvent::RepositoryUpdated(_, _, true)
            ) {
                this.bind_repository(cx);
            }
        })];
        if let Some(workspace) = handle.upgrade() {
            observers.push(cx.observe(&workspace, |_, _, cx| cx.notify()));
        }
        let mut this = Self {
            workspace: handle,
            git,
            repository: None,
            repository_observer: None,
            editor_observer: None,
            _observers: observers,
        };
        this.bind_repository(cx);
        this
    }
    fn bind_repository(&mut self, cx: &mut Context<Self>) {
        self.repository = self.git.read(cx).active_repository();
        self.repository_observer = self
            .repository
            .as_ref()
            .map(|repository| cx.observe(repository, |_, _, cx| cx.notify()));
        cx.notify();
    }
    pub(super) fn labels(&self, cx: &App) -> (String, bool) {
        let branch = self
            .repository
            .as_ref()
            .map(|repository| repository.read(cx))
            .and_then(|repository| {
                repository
                    .branch
                    .as_ref()
                    .map(|branch| branch.name().to_string())
                    .or_else(|| {
                        repository
                            .head_commit
                            .as_ref()
                            .map(|commit| commit.sha.chars().take(8).collect())
                    })
            })
            .unwrap_or_else(|| "Local project".into());
        let dirty = self
            .workspace
            .read_with(cx, |workspace, cx| {
                workspace
                    .panes()
                    .iter()
                    .any(|pane| pane.read(cx).items().any(|item| item.is_dirty(cx)))
            })
            .unwrap_or(false);
        (branch, dirty)
    }
}
impl StatusItemView for ProjectState {
    fn set_active_pane_item(
        &mut self,
        item: Option<&dyn ItemHandle>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editor_observer = item
            .and_then(|item| item.downcast::<Editor>())
            .map(|editor| cx.observe(&editor, |_, _, cx| cx.notify()));
        cx.notify();
    }
    fn hide_setting(&self, _: &App) -> Option<HideStatusItem> {
        None
    }
}
impl Render for ProjectState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (branch, dirty) = self.labels(cx);
        let workspace = self.workspace.clone();
        h_flex()
            .gap_2()
            .min_w_0()
            .child(
                Button::new(
                    "zedx-status-branch",
                    util::truncate_and_trailoff(
                        &branch,
                        if window.viewport_size().width < px(560.0) {
                            12
                        } else {
                            35
                        },
                    ),
                )
                .label_size(ui::zedx::caption_size())
                .start_icon(Icon::new(IconName::GitBranch).size(IconSize::Small))
                .tooltip(Tooltip::text(branch))
                .on_click(move |_, window, cx| {
                    if let Some(workspace) = workspace.upgrade() {
                        window.focus(&workspace.read(cx).focus_handle(cx), cx);
                        window.dispatch_action(zed_actions::git::Branch.boxed_clone(), cx);
                    }
                }),
            )
            .when(window.viewport_size().width >= px(700.0), |row| {
                row.child(
                    h_flex()
                        .gap_1()
                        .debug_selector(|| "zedx-save-state".into())
                        .child(
                            Icon::new(if dirty {
                                IconName::Circle
                            } else {
                                IconName::Check
                            })
                            .size(IconSize::Small)
                            .color(if dirty {
                                Color::Warning
                            } else {
                                Color::Success
                            }),
                        )
                        .child(
                            Label::new(if dirty {
                                "Unsaved changes"
                            } else {
                                "All changes saved"
                            })
                            .size(ui::zedx::caption_size())
                            .color(Color::Muted),
                        ),
                )
            })
    }
}

pub(crate) struct Indentation {
    workspace: WeakEntity<Workspace>,
    editor: Option<Entity<Editor>>,
    observer: Option<Subscription>,
}
impl Indentation {
    pub(crate) fn new(workspace: &Workspace) -> Self {
        Self {
            workspace: workspace.weak_handle(),
            editor: None,
            observer: None,
        }
    }
    pub(super) fn label(&self, cx: &App) -> Option<String> {
        let editor = self.editor.as_ref()?.read(cx);
        let settings = editor.buffer().read(cx).language_settings(cx);
        Some(if settings.hard_tabs {
            format!("Tabs: {}", settings.tab_size)
        } else {
            format!("{} spaces", settings.tab_size)
        })
    }
}
impl StatusItemView for Indentation {
    fn set_active_pane_item(
        &mut self,
        item: Option<&dyn ItemHandle>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editor = item.and_then(|item| item.downcast::<Editor>());
        self.observer = self
            .editor
            .as_ref()
            .map(|editor| cx.observe(editor, |_, _, cx| cx.notify()));
        cx.notify();
    }
    fn hide_setting(&self, _: &App) -> Option<HideStatusItem> {
        None
    }
}
impl Render for Indentation {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().when_some(self.label(cx), |row, label| {
            row.child(
                Button::new("zedx-indentation", label.clone())
                    .label_size(ui::zedx::caption_size())
                    .tab_index(0isize)
                    .tooltip(Tooltip::text("Current file indentation"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.workspace
                            .update(cx, |workspace, cx| {
                                workspace.show_toast(
                                    Toast::new(
                                        NotificationId::unique::<Indentation>(),
                                        format!("Indentation: {label}"),
                                    )
                                    .autohide(),
                                    cx,
                                )
                            })
                            .log_err();
                    })),
            )
        })
    }
}

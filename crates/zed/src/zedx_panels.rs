use editor::{Editor, SelectionEffects, scroll::Autoscroll};
use gpui::{
    Action, App, AsyncWindowContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Render,
    Subscription, Task, WeakEntity, Window, uniform_list,
};
use language::{Anchor, Buffer};
use outline_panel::OutlinePanel;
use project::{Project, ProjectPath};
use std::{collections::HashSet, ops::Range};
use ui::{ListItem, ListItemSpacing, Tooltip, prelude::*};
use util::ResultExt;
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
    zedx_actions::{ToggleBranches, ToggleProblems, ToggleStructure},
};

pub(crate) fn init(cx: &mut App) {
    find::bind_keys(cx);
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(open_find);
        workspace.register_action(
            |workspace, _: &workspace::zedx_actions::ToggleRun, window, cx| {
                workspace.toggle_panel_focus::<Run>(window, cx);
            },
        );
        workspace.register_action(
            |workspace, _: &workspace::zedx_actions::ToggleBranches, window, cx| {
                workspace.toggle_panel_focus::<Branches>(window, cx);
            },
        );
        workspace.register_action(|workspace, _: &ToggleStructure, window, cx| {
            workspace.toggle_panel_focus::<Structure>(window, cx);
        });
        workspace.register_action(|workspace, _: &ToggleProblems, window, cx| {
            workspace.toggle_panel_focus::<Problems>(window, cx);
        });
        workspace.register_action(
            |workspace, _: &workspace::zedx_actions::ShowGitHistory, window, cx| {
                workspace.focus_panel::<git_ui::git_panel::GitPanel>(window, cx);
                if let Some(panel) = workspace.panel::<git_ui::git_panel::GitPanel>(cx) {
                    panel.update(cx, |panel, cx| panel.show_history(window, cx));
                }
            },
        );
    })
    .detach();
}

pub(crate) async fn initialize(
    workspace: WeakEntity<Workspace>,
    mut cx: AsyncWindowContext,
) -> anyhow::Result<()> {
    let outline = OutlinePanel::load(workspace.clone(), cx.clone()).await?;
    workspace.update_in(&mut cx, |workspace, window, cx| {
        let structure = cx.new(|cx| {
            let subscription = cx.subscribe(&outline, |_, _, event: &PanelEvent, cx| {
                cx.emit(match event {
                    PanelEvent::ZoomIn => PanelEvent::ZoomIn,
                    PanelEvent::ZoomOut => PanelEvent::ZoomOut,
                    PanelEvent::Activate => PanelEvent::Activate,
                    PanelEvent::Close => PanelEvent::Close,
                });
            });
            Structure {
                outline,
                focus: cx.focus_handle(),
                zoomed: false,
                _subscription: subscription,
            }
        });
        let find = cx.new(|cx| Find::new(workspace, window, cx));
        workspace.add_panel(find, window, cx);
        let problems = cx.new(|cx| Problems::new(workspace, window, cx));
        workspace.add_panel(structure, window, cx);
        workspace.add_panel(problems, window, cx);
        let run = cx.new(|cx| Run::new(workspace, window, cx));
        workspace.add_panel(run, window, cx);
        let branches = cx.new(|cx| Branches::new(workspace, window, cx));
        workspace.add_panel(branches, window, cx);
    })?;
    Ok(())
}

macro_rules! fixed_panel {
    ($type:ty, $name:literal, $key:literal, $position:expr, $size:expr, $icon:expr, $toggle:expr, $priority:expr) => {
        impl Panel for $type {
            fn persistent_name() -> &'static str {
                $name
            }
            fn panel_key() -> &'static str {
                $key
            }
            fn position(&self, _: &Window, _: &App) -> DockPosition {
                $position
            }
            fn position_is_valid(&self, position: DockPosition) -> bool {
                position == $position
            }
            fn set_position(&mut self, _: DockPosition, _: &mut Window, _: &mut Context<Self>) {}
            fn default_size(&self, _: &Window, _: &App) -> Pixels {
                px($size)
            }
            fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
                Some($icon)
            }
            fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
                Some($name)
            }
            fn toggle_action(&self) -> Box<dyn Action> {
                $toggle.boxed_clone()
            }
            fn activation_focus_handle(&self, cx: &App) -> FocusHandle {
                self.activation_focus(cx)
            }
            fn set_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
                self.on_active(active, window, cx);
            }
            fn is_zoomed(&self, _: &Window, _: &App) -> bool {
                self.zoomed
            }
            fn set_zoomed(&mut self, zoomed: bool, _: &mut Window, cx: &mut Context<Self>) {
                self.zoomed = zoomed;
                cx.notify();
            }
            fn activation_priority(&self) -> u32 {
                $priority
            }
            fn icon_label(&self, _: &Window, cx: &App) -> Option<String> {
                self.badge(cx)
            }
        }
        impl Focusable for $type {
            fn focus_handle(&self, _: &App) -> FocusHandle {
                self.focus.clone()
            }
        }
        impl EventEmitter<PanelEvent> for $type {}
    };
}

#[path = "zedx_run_panel.rs"]
mod run;
use run::Run;
#[path = "zedx_find_panel.rs"]
mod find;
use find::Find;

pub(crate) fn open_find(
    workspace: &mut Workspace,
    action: &workspace::zedx_actions::FindInFiles,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    if let Some(panel) = workspace.panel::<Find>(cx) {
        panel.update(cx, |panel, cx| panel.configure(action, window, cx));
        workspace.focus_panel::<Find>(window, cx);
    }
}

pub(crate) struct Structure {
    _subscription: Subscription,
    outline: Entity<OutlinePanel>,
    focus: FocusHandle,
    zoomed: bool,
}
impl Structure {
    fn activation_focus(&self, cx: &App) -> FocusHandle {
        self.outline.read(cx).activation_focus_handle(cx)
    }
    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.outline
            .update(cx, |outline, cx| outline.set_active(active, window, cx));
    }
    fn badge(&self, _: &App) -> Option<String> {
        None
    }
}
fixed_panel!(
    Structure,
    "Structure",
    "ZedXStructure",
    DockPosition::Left,
    265.0,
    IconName::ListTree,
    ToggleStructure,
    7
);
impl Render for Structure {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .child(
                ui::zedx::tool_window_head("structure", ui::zedx::Icon::Structure, "Structure", cx)
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                ui::zedx::tool_button(
                                    "structure-collapse",
                                    ui::zedx::Icon::Collapse,
                                    "Collapse all",
                                    cx,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        window.focus(&this.activation_focus(cx), cx);
                                        window.dispatch_action(
                                            outline_panel::CollapseAllEntries.boxed_clone(),
                                            cx,
                                        );
                                    },
                                )),
                            )
                            .child(
                                ui::zedx::tool_button(
                                    "structure-hide",
                                    ui::zedx::Icon::Minus,
                                    "Hide Structure",
                                    cx,
                                )
                                .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Close))),
                            ),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.outline.clone()))
    }
}

#[derive(Clone)]
struct Problem {
    path: ProjectPath,
    buffer: Entity<Buffer>,
    range: Range<Anchor>,
    message: String,
    location: String,
    error: bool,
}
pub(crate) struct Problems {
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    focus: FocusHandle,
    zoomed: bool,
    active: bool,
    rows: Vec<Problem>,
    error: Option<String>,
    refresh: Task<()>,
    _subscription: Subscription,
}
impl Problems {
    fn new(workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project = workspace.project().clone();
        let subscription =
            cx.subscribe_in(&project, window, |this: &mut Self, _, event, window, cx| {
                if matches!(event, project::Event::DiagnosticsUpdated { .. }) {
                    if this.active {
                        this.refresh(window, cx);
                    }
                    cx.notify();
                }
            });
        Self {
            workspace: workspace.weak_handle(),
            project,
            focus: cx.focus_handle(),
            zoomed: false,
            active: false,
            rows: Vec::new(),
            error: None,
            refresh: Task::ready(()),
            _subscription: subscription,
        }
    }
    fn activation_focus(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active {
            self.refresh(window, cx);
        }
    }
    fn badge(&self, cx: &App) -> Option<String> {
        let summary = self.project.read(cx).diagnostic_summary(false, cx);
        let count = summary.error_count + summary.warning_count;
        (count > 0).then(|| count.to_string())
    }
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut seen = HashSet::new();
        let paths = self
            .project
            .read(cx)
            .diagnostic_summaries(false, cx)
            .map(|(path, _, _)| path)
            .filter(|path| seen.insert(path.clone()))
            .collect::<Vec<_>>();
        let project = self.project.clone();
        self.refresh = cx.spawn_in(window, async move |panel, cx| {
            let mut rows = Vec::new();
            let mut error = None;
            for path in paths {
                let task = project.update(cx, |project, cx| project.open_buffer(path.clone(), cx));
                match task.await {
                    Ok(buffer) => buffer.read_with(cx, |buffer_data, _| {
                        let snapshot = buffer_data.snapshot();
                        for entry in snapshot.diagnostic_entries_in_range(0..snapshot.len(), false)
                        {
                            if !entry.diagnostic.is_primary
                                || !matches!(
                                    entry.diagnostic.severity,
                                    lsp::DiagnosticSeverity::ERROR
                                        | lsp::DiagnosticSeverity::WARNING
                                )
                            {
                                continue;
                            }
                            let point: language::Point =
                                snapshot.summary_for_anchor(&entry.range.start);
                            rows.push(Problem {
                                path: path.clone(),
                                buffer: buffer.clone(),
                                range: entry.range.clone(),
                                message: entry.diagnostic.message.as_str().to_string(),
                                location: format!(
                                    "{}:{}:{}",
                                    path.path.as_unix_str(),
                                    point.row + 1,
                                    point.column + 1
                                ),
                                error: entry.diagnostic.severity == lsp::DiagnosticSeverity::ERROR,
                            });
                        }
                    }),
                    Err(problem) => error = Some(format!("Cannot load diagnostics: {problem}")),
                }
            }
            rows.sort_by(|left, right| {
                right
                    .error
                    .cmp(&left.error)
                    .then_with(|| left.location.cmp(&right.location))
            });
            panel
                .update(cx, |panel, cx| {
                    panel.rows = rows;
                    panel.error = error;
                    cx.notify();
                })
                .log_err();
        });
    }
    fn open(&self, row: Problem, window: &mut Window, cx: &mut Context<Self>) {
        open_source(&self.workspace, row.buffer, row.range, window, cx);
    }
}
fn open_source(
    workspace: &WeakEntity<Workspace>,
    buffer: Entity<Buffer>,
    range: Range<Anchor>,
    window: &mut Window,
    cx: &mut App,
) {
    workspace
        .update(cx, |workspace, cx| {
            let editor = workspace
                .open_project_item::<Editor>(None, buffer, true, true, true, true, window, cx);
            editor.update(cx, |editor, cx| {
                let snapshot = editor.buffer().read(cx).snapshot(cx);
                if let (Some(start), Some(end)) = (
                    snapshot.anchor_in_buffer(range.start),
                    snapshot.anchor_in_buffer(range.end),
                ) {
                    editor.change_selections(
                        SelectionEffects::scroll(Autoscroll::center()).unfold(),
                        window,
                        cx,
                        |selections| selections.select_ranges([start..end]),
                    );
                }
            });
        })
        .log_err();
}
fixed_panel!(
    Problems,
    "Problems",
    "ZedXProblems",
    DockPosition::Bottom,
    202.0,
    IconName::Warning,
    ToggleProblems,
    4
);
impl Render for Problems {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .child(
                ui::zedx::tool_window_head("problems", ui::zedx::Icon::Warning, "Problems", cx)
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Label::new(self.rows.len().to_string())
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            )
                            .child(
                                div().debug_selector(|| "zedx-problems-hide".into()).child(
                                    ui::zedx::tool_button(
                                        "zedx-problems-hide",
                                        ui::zedx::Icon::Minus,
                                        "Hide Problems",
                                        cx,
                                    )
                                    .on_click(
                                        cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Close)),
                                    ),
                                ),
                            ),
                    ),
            )
            .when_some(self.error.clone(), |column, error| {
                column.child(Label::new(error).color(Color::Warning))
            })
            .when(self.rows.is_empty(), |column| {
                column.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Label::new("No problems in workspace").color(Color::Muted)),
                )
            })
            .when(!self.rows.is_empty(), |column| {
                column.child(
                    uniform_list(
                        "zedx-problems-list",
                        self.rows.len(),
                        cx.processor(|this, range: Range<usize>, _, cx| {
                            range
                                .filter_map(|index| {
                                    let row = this.rows.get(index)?.clone();
                                    Some(
                                        ListItem::new(index)
                                            .spacing(ListItemSpacing::Dense)
                                            .height(px(27.0))
                                            .child(
                                                h_flex()
                                                    .min_w_0()
                                                    .gap_2()
                                                    .child(
                                                        Icon::new(if row.error {
                                                            IconName::XCircle
                                                        } else {
                                                            IconName::Warning
                                                        })
                                                        .size(IconSize::Small)
                                                        .color(if row.error {
                                                            Color::Error
                                                        } else {
                                                            Color::Warning
                                                        }),
                                                    )
                                                    .child(
                                                        Label::new(row.message.clone())
                                                            .size(LabelSize::Small)
                                                            .truncate(),
                                                    )
                                                    .child(
                                                        Label::new(row.location.clone())
                                                            .size(LabelSize::XSmall)
                                                            .color(Color::Muted)
                                                            .truncate(),
                                                    ),
                                            )
                                            .tooltip(Tooltip::text(format!(
                                                "{}\n{}",
                                                row.message,
                                                row.path.path.as_unix_str()
                                            )))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.open(row.clone(), window, cx)
                                            })),
                                    )
                                })
                                .collect::<Vec<_>>()
                        }),
                    )
                    .flex_1(),
                )
            })
    }
}

pub(crate) struct Branches {
    workspace: WeakEntity<Workspace>,
    branches: Entity<git_ui::branch_picker::BranchList>,
    focus: FocusHandle,
    zoomed: bool,
    _subscription: Subscription,
}
impl Branches {
    fn new(workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace_handle = workspace.weak_handle();
        let git = workspace.project().read(cx).git_store().clone();
        let repository = Self::active_repository(&git, cx);
        let branches = Self::build(&workspace_handle, repository, window, cx);
        let git_handle = git.clone();
        let subscription = cx.subscribe_in(
            &git,
            window,
            move |this: &mut Self, _, event, window, cx| {
                if matches!(
                    event,
                    project::git_store::GitStoreEvent::ActiveRepositoryChanged(_)
                        | project::git_store::GitStoreEvent::RepositoryAdded
                        | project::git_store::GitStoreEvent::RepositoryRemoved(_)
                        | project::git_store::GitStoreEvent::RepositoryUpdated(_, _, true)
                ) {
                    this.rebuild(&git_handle, window, cx);
                }
            },
        );
        Self {
            workspace: workspace_handle,
            branches,
            focus: cx.focus_handle(),
            zoomed: false,
            _subscription: subscription,
        }
    }

    fn active_repository(
        git: &Entity<project::git_store::GitStore>,
        cx: &App,
    ) -> Option<Entity<project::git_store::Repository>> {
        git.read(cx).active_repository()
    }

    fn build(
        workspace: &WeakEntity<Workspace>,
        repository: Option<Entity<project::git_store::Repository>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<git_ui::branch_picker::BranchList> {
        cx.new(|cx| {
            // The embedded footer carries the prototype's branch tools: create from the default
            // branch, delete, and switch.
            git_ui::branch_picker::create_embedded(
                workspace.clone(),
                repository,
                rems(40.0),
                true,
                window,
                cx,
            )
        })
    }

    fn rebuild(
        &mut self,
        git: &Entity<project::git_store::GitStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let repository = Self::active_repository(git, cx);
        self.branches = Self::build(&self.workspace, repository, window, cx);
        cx.notify();
    }

    fn activation_focus(&self, cx: &App) -> FocusHandle {
        self.branches.read(cx).picker.focus_handle(cx)
    }
    fn on_active(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) {}
    fn badge(&self, _: &App) -> Option<String> {
        None
    }
}
fixed_panel!(
    Branches,
    "Branches",
    "ZedXBranches",
    DockPosition::Left,
    240.0,
    IconName::GitBranch,
    ToggleBranches,
    5
);
impl Render for Branches {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .child(
                ui::zedx::tool_window_head("branches", ui::zedx::Icon::Git, "Branches", cx)
                    .justify_between()
                    .child(
                        div().debug_selector(|| "zedx-branches-hide".into()).child(
                            ui::zedx::tool_button(
                                "zedx-branches-hide",
                                ui::zedx::Icon::Minus,
                                "Hide Branches",
                                cx,
                            )
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Close))),
                        ),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.branches.clone()))
    }
}

#[cfg(test)]
#[path = "zedx_panels_tests.rs"]
mod tests;

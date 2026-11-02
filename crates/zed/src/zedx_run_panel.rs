use super::*;
use gpui::EntityId;
use task::RevealStrategy;
use terminal::TaskStatus;
use terminal_view::{TerminalView, terminal_panel::TerminalPanel};
use workspace::zedx_actions::ToggleRun;

pub(crate) struct Run {
    workspace: WeakEntity<Workspace>,
    focus: FocusHandle,
    zoomed: bool,
    current: Option<Entity<TerminalView>>,
    last_task: Option<(EntityId, TaskStatus)>,
    terminal_panel: Option<Entity<TerminalPanel>>,
    panel_observer: Option<Subscription>,
    view_observer: Option<Subscription>,
    model_observer: Option<Subscription>,
    _workspace_observer: Option<Subscription>,
}
impl Run {
    pub(super) fn new(workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let handle = workspace.weak_handle();
        let observer = handle.upgrade().map(|workspace| {
            cx.subscribe_in(
                &workspace,
                window,
                |this: &mut Self, _, event, window, cx| {
                    if matches!(event, workspace::Event::PanelAdded(_))
                        && this.terminal_panel.is_none()
                    {
                        this.attach(window, cx);
                    }
                },
            )
        });
        let mut this = Self {
            workspace: handle,
            focus: cx.focus_handle(),
            zoomed: false,
            current: None,
            last_task: None,
            terminal_panel: workspace.panel::<TerminalPanel>(cx),
            panel_observer: None,
            view_observer: None,
            model_observer: None,
            _workspace_observer: observer,
        };
        this.panel_observer = this.terminal_panel.as_ref().map(|panel| {
            cx.observe_in(panel, window, |this, _, window, cx| {
                this.refresh(true, window, cx)
            })
        });
        this.refresh(false, window, cx);
        this
    }
    fn attach(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.terminal_panel = self
            .workspace
            .read_with(cx, |workspace, cx| workspace.panel::<TerminalPanel>(cx))
            .ok()
            .flatten();
        self.panel_observer = self.terminal_panel.as_ref().map(|panel| {
            cx.observe_in(panel, window, |this, _, window, cx| {
                this.refresh(true, window, cx)
            })
        });
        self.refresh(false, window, cx);
    }
    fn latest_task(&self, cx: &App) -> Option<Entity<TerminalView>> {
        let panel = self.terminal_panel.as_ref()?.read(cx);
        if let Some(view) = panel
            .pane()?
            .read(cx)
            .active_item()
            .and_then(|item| item.downcast::<TerminalView>())
            && view.read(cx).terminal().read(cx).task().is_some()
        {
            return Some(view);
        }
        panel
            .panes()
            .into_iter()
            .flat_map(|pane| {
                pane.read(cx)
                    .items_of_type::<TerminalView>()
                    .collect::<Vec<_>>()
            })
            .filter(|view| view.read(cx).terminal().read(cx).task().is_some())
            .last()
    }
    fn refresh(&mut self, reveal: bool, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.latest_task(cx);
        if current.as_ref().map(Entity::entity_id) != self.current.as_ref().map(Entity::entity_id) {
            self.view_observer = current
                .as_ref()
                .map(|view| cx.observe(view, |_, _, cx| cx.notify()));
            self.model_observer = current.as_ref().map(|view| {
                let terminal = view.read(cx).terminal().clone();
                cx.observe_in(&terminal, window, |this, _, window, cx| {
                    this.refresh(true, window, cx)
                })
            });
            self.current = current;
        }
        let task = self.current.as_ref().and_then(|view| {
            let view = view.read(cx);
            let terminal = view.terminal();
            terminal
                .read(cx)
                .task()
                .map(|task| (terminal.entity_id(), task.status, task.spawned_task.reveal))
        });
        if let Some((id, status, strategy)) = task {
            let newly_started = self.last_task.is_none_or(|(previous, previous_status)| {
                previous != id
                    || (status == TaskStatus::Running && previous_status != TaskStatus::Running)
            });
            self.last_task = Some((id, status));
            if reveal && newly_started && strategy != RevealStrategy::Never {
                let workspace = self.workspace.clone();
                cx.spawn_in(window, async move |_, cx| {
                    workspace
                        .update_in(cx, |workspace, window, cx| {
                            if strategy == RevealStrategy::Always
                                && !workspace.has_active_modal(window, cx)
                            {
                                workspace.focus_panel::<Run>(window, cx);
                            } else {
                                workspace.open_panel::<Run>(window, cx);
                            }
                        })
                        .log_err();
                })
                .detach();
            }
        }
        cx.notify();
    }
    fn activation_focus(&self, cx: &App) -> FocusHandle {
        self.current
            .as_ref()
            .map(|view| view.focus_handle(cx))
            .unwrap_or_else(|| self.focus.clone())
    }
    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        if active {
            self.refresh(false, window, cx);
        }
    }
    fn badge(&self, _: &App) -> Option<String> {
        None
    }
    pub(super) fn task_label(&self, cx: &App) -> Option<(String, &'static str, bool)> {
        let terminal = self.current.as_ref()?.read(cx).terminal().read(cx);
        let task = terminal.task()?;
        Some((
            task.spawned_task.full_label.to_string(),
            match task.status {
                TaskStatus::Running => "Running",
                TaskStatus::Completed { success: true } => "Finished successfully",
                TaskStatus::Completed { success: false } => "Failed",
                TaskStatus::Unknown => "Stopped",
            },
            task.status == TaskStatus::Completed { success: false },
        ))
    }
}
fixed_panel!(
    Run,
    "Run",
    "ZedXRun",
    DockPosition::Bottom,
    202.0,
    IconName::TriangleRight,
    ToggleRun,
    3
);
impl Render for Run {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .when_some(self.task_label(cx), |column, (name, state, failed)| {
                column.child(
                    h_flex()
                        .h(px(25.0))
                        .px_2()
                        .gap_2()
                        .child(Label::new(name).size(ui::zedx::label_size()).truncate())
                        .child(
                            Label::new(state)
                                .size(ui::zedx::caption_size())
                                .color(if failed { Color::Error } else { Color::Muted }),
                        ),
                )
            })
            .when_some(self.current.clone(), |column, view| {
                column.child(div().flex_1().min_h_0().child(view))
            })
            .when(self.current.is_none(), |column| {
                column.child(
                    v_flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(Label::new("No task has run yet").color(Color::Muted))
                        .child(
                            Button::new("zedx-choose-task", "Choose run configuration").on_click(
                                |_, window, cx| {
                                    window.dispatch_action(
                                        zed_actions::Spawn::modal().boxed_clone(),
                                        cx,
                                    )
                                },
                            ),
                        ),
                )
            })
    }
}

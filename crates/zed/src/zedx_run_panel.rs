use super::*;
use gpui::EntityId;
use task::RevealStrategy;
use terminal::TaskStatus;
use terminal_view::{TerminalView, terminal_panel::TerminalPanel};
use ui::ButtonLike;
use workspace::zedx_actions::ToggleRun;

fn selected_task(
    current: Option<EntityId>,
    previous: &[EntityId],
    tasks: &[EntityId],
) -> Option<EntityId> {
    tasks
        .iter()
        .rev()
        .find(|id| !previous.contains(id))
        .copied()
        .or_else(|| current.filter(|id| tasks.contains(id)))
        .or_else(|| tasks.last().copied())
}

pub(crate) struct Run {
    workspace: WeakEntity<Workspace>,
    focus: FocusHandle,
    zoomed: bool,
    rerun_reveal: bool,
    current: Option<Entity<TerminalView>>,
    last_task: Option<(EntityId, TaskStatus)>,
    terminal_panel: Option<Entity<TerminalPanel>>,
    panel_observer: Option<Subscription>,
    view_observer: Option<Subscription>,
    model_observer: Option<Subscription>,
    _workspace_observer: Option<Subscription>,
    workspace_panes: Vec<Entity<workspace::Pane>>,
    task_sources: Vec<EntityId>,
    task_observers: Vec<Subscription>,
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
                    } else if matches!(
                        event,
                        workspace::Event::PaneAdded(_)
                            | workspace::Event::PaneRemoved
                            | workspace::Event::ItemAdded { .. }
                            | workspace::Event::ItemRemoved { .. }
                    ) {
                        cx.defer_in(window, |this, window, cx| {
                            if let Some(workspace) = this.workspace.upgrade() {
                                this.workspace_panes = workspace.read(cx).panes().to_vec();
                            }
                            this.refresh(true, window, cx);
                        });
                    }
                },
            )
        });
        let mut this = Self {
            workspace: handle,
            focus: cx.focus_handle(),
            zoomed: false,
            rerun_reveal: false,
            current: None,
            last_task: None,
            terminal_panel: workspace.panel::<TerminalPanel>(cx),
            panel_observer: None,
            view_observer: None,
            model_observer: None,
            _workspace_observer: observer,
            workspace_panes: workspace.panes().to_vec(),
            task_sources: Vec::new(),
            task_observers: Vec::new(),
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
    fn task_views(&self, cx: &App) -> Vec<Entity<TerminalView>> {
        let mut panes = self.workspace_panes.clone();
        if let Some(panel) = &self.terminal_panel {
            panes.extend(panel.read(cx).panes().into_iter().cloned());
        }
        panes
            .into_iter()
            .flat_map(|pane| {
                pane.read(cx)
                    .items_of_type::<TerminalView>()
                    .collect::<Vec<_>>()
            })
            .filter(|view| view.read(cx).terminal().read(cx).task().is_some())
            .collect()
    }
    fn select_task(
        &mut self,
        view: Entity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.view_observer = Some(cx.observe_in(&view, window, |this, _, window, cx| {
            this.refresh(true, window, cx)
        }));
        self.current = Some(view);
        self.refresh(false, window, cx);
        window.focus(&self.activation_focus(cx), cx);
    }
    fn refresh(&mut self, reveal: bool, window: &mut Window, cx: &mut Context<Self>) {
        let tasks = self.task_views(cx);
        let mut sources = tasks.iter().map(Entity::entity_id).collect::<Vec<_>>();
        sources.extend(
            tasks
                .iter()
                .map(|view| view.read(cx).terminal().entity_id()),
        );
        sources.extend(self.workspace_panes.iter().map(Entity::entity_id));
        if let Some(panel) = &self.terminal_panel {
            sources.extend(panel.read(cx).panes().into_iter().map(Entity::entity_id));
        }
        let selected = selected_task(
            self.current.as_ref().map(Entity::entity_id),
            &self.task_sources,
            &tasks.iter().map(Entity::entity_id).collect::<Vec<_>>(),
        );
        let current = tasks
            .iter()
            .find(|view| Some(view.entity_id()) == selected)
            .cloned();
        if sources != self.task_sources {
            self.task_sources = sources;
            let mut observers = Vec::new();
            {
                let mut panes = self.workspace_panes.clone();
                if let Some(panel) = &self.terminal_panel {
                    panes.extend(panel.read(cx).panes().into_iter().cloned());
                }
                for pane in panes {
                    observers.push(cx.observe_in(&pane, window, |this, _, window, cx| {
                        this.refresh(true, window, cx)
                    }));
                }
            }
            for view in &tasks {
                observers.push(cx.observe_in(view, window, |this, _, window, cx| {
                    this.refresh(true, window, cx)
                }));
                let terminal = view.read(cx).terminal().clone();
                observers.push(cx.observe_in(&terminal, window, |this, _, window, cx| {
                    this.refresh(true, window, cx)
                }));
            }
            self.task_observers = observers;
        }
        if current.as_ref().map(Entity::entity_id) != self.current.as_ref().map(Entity::entity_id) {
            self.view_observer = current.as_ref().map(|view| {
                cx.observe_in(view, window, |this, _, window, cx| {
                    this.refresh(true, window, cx)
                })
            });
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
    fn rerun(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = &self.current else {
            return;
        };
        self.rerun_reveal = true;
        view.focus_handle(cx)
            .dispatch_action(&terminal_view::RerunTask, window, cx);
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
        } else if std::mem::take(&mut self.rerun_reveal) {
            // Native task replacement reveals Terminal after publishing its new model.
            let workspace = self.workspace.clone();
            cx.spawn_in(window, async move |_, cx| {
                workspace
                    .update_in(cx, |workspace, window, cx| {
                        workspace.focus_panel::<Run>(window, cx)
                    })
                    .log_err();
            })
            .detach();
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let font = theme_settings::setup_ui_font(window, cx);
        let tasks = self.task_views(cx);
        let current = self.current.clone();
        let running = current.as_ref().is_some_and(|view| {
            view.read(cx)
                .terminal()
                .read(cx)
                .task()
                .is_some_and(|task| task.status == TaskStatus::Running)
        });
        v_flex()
            .font(font)
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .track_focus(&self.focus)
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .flex_none()
                    .h(gpui::rems(2.5))
                    .px_1()
                    .gap_1()
                    .debug_selector(|| "zedx-run-task-tabs".into())
                    .child(
                        h_flex()
                            .id("zedx-run-tabs-scroll")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_x_scroll()
                            .children(tasks.into_iter().map(|view| {
                                let selected = current
                                    .as_ref()
                                    .is_some_and(|current| current.entity_id() == view.entity_id());
                                let terminal = view.read(cx).terminal().read(cx);
                                let task = terminal.task();
                                let label = task
                                    .map(|task| task.spawned_task.full_label.to_string())
                                    .unwrap_or_default();
                                let failed = task.is_some_and(|task| {
                                    task.status == TaskStatus::Completed { success: false }
                                });
                                let id = view.entity_id();
                                h_flex()
                                    .id(("zedx-run-task", id))
                                    .debug_selector(move || format!("zedx-run-task-{id}"))
                                    .min_w_0()
                                    .flex_shrink_0()
                                    .h_full()
                                    .px_2()
                                    .gap_2()
                                    .cursor_pointer()
                                    .max_w(gpui::rems(24.0))
                                    .overflow_hidden()
                                    .when(selected, |tab| {
                                        tab.bg(cx.theme().colors().element_selected)
                                    })
                                    .hover(|tab| tab.bg(cx.theme().colors().element_hover))
                                    .child(
                                        Icon::new(if failed {
                                            IconName::XCircle
                                        } else {
                                            IconName::TriangleRight
                                        })
                                        .size(IconSize::Small)
                                        .color(if failed { Color::Error } else { Color::Success }),
                                    )
                                    .child(
                                        Label::new(label).size(ui::zedx::label_size()).truncate(),
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.select_task(view.clone(), window, cx)
                                    }))
                            })),
                    )
                    .child(
                        IconButton::new("zedx-run-choose", IconName::Plus)
                            .tooltip(Tooltip::text("Choose run configuration"))
                            .on_click(|_, window, cx| {
                                window
                                    .dispatch_action(zed_actions::Spawn::modal().boxed_clone(), cx)
                            }),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        v_flex()
                            .w(px(36.0))
                            .h_full()
                            .flex_none()
                            .items_center()
                            .gap_1()
                            .py_1()
                            .debug_selector(|| "zedx-run-actions".into())
                            .child(
                                IconButton::new("zedx-run-rerun", IconName::Rerun)
                                    .icon_color(Color::Success)
                                    .disabled(current.is_none())
                                    .tooltip(Tooltip::text("Rerun selected task"))
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.rerun(window, cx)),
                                    ),
                            )
                            .child(
                                ButtonLike::new("zedx-run-stop")
                                    .width(px(28.0))
                                    .height(px(28.0).into())
                                    .disabled(!running)
                                    .aria_label("Stop selected task")
                                    .tooltip(Tooltip::text("Stop selected task"))
                                    .child(div().size(px(11.0)).rounded(px(1.0)).bg(if running {
                                        gpui::rgb(0xdb5c5c).into()
                                    } else {
                                        cx.theme().colors().icon_disabled
                                    }))
                                    .on_click({
                                        let current = current.clone();
                                        move |_, _, cx| {
                                            if let Some(view) = &current {
                                                view.read(cx)
                                                    .terminal()
                                                    .clone()
                                                    .update(cx, |terminal, _| {
                                                        terminal.kill_active_task()
                                                    });
                                            }
                                        }
                                    }),
                            )
                            .child(
                                IconButton::new("zedx-run-clear", IconName::Trash)
                                    .disabled(current.is_none())
                                    .tooltip(Tooltip::text("Clear console"))
                                    .on_click({
                                        let current = current.clone();
                                        move |_, _, cx| {
                                            if let Some(view) = &current {
                                                view.read(cx)
                                                    .terminal()
                                                    .clone()
                                                    .update(cx, |terminal, _| terminal.clear());
                                            }
                                        }
                                    }),
                            )
                            .child(
                                IconButton::new("zedx-run-bottom", IconName::ArrowDown)
                                    .disabled(current.is_none())
                                    .tooltip(Tooltip::text("Scroll to end"))
                                    .on_click({
                                        let current = current.clone();
                                        move |_, _, cx| {
                                            if let Some(view) = &current {
                                                view.read(cx)
                                                    .terminal()
                                                    .clone()
                                                    .update(cx, |terminal, _| {
                                                        terminal.scroll_to_bottom()
                                                    });
                                            }
                                        }
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .min_h_0()
                            .when_some(self.task_label(cx), |column, (name, state, failed)| {
                                column.child(
                                    h_flex()
                                        .w_full()
                                        .min_w_0()
                                        .h(gpui::rems(2.25))
                                        .flex_none()
                                        .px_2()
                                        .gap_2()
                                        .child(
                                            div().flex_1().min_w_0().overflow_hidden().child(
                                                Label::new(name)
                                                    .size(ui::zedx::label_size())
                                                    .truncate(),
                                            ),
                                        )
                                        .child(
                                            Label::new(state).size(ui::zedx::caption_size()).color(
                                                if failed {
                                                    Color::Error
                                                } else if running {
                                                    Color::Success
                                                } else {
                                                    Color::Muted
                                                },
                                            ),
                                        ),
                                )
                            })
                            .when_some(current.clone(), |column, view| {
                                column.child(div().flex_1().min_h_0().w_full().child(view))
                            })
                            .when(current.is_none(), |column| {
                                column.child(
                                    v_flex()
                                        .flex_1()
                                        .w_full()
                                        .items_center()
                                        .justify_center()
                                        .gap_2()
                                        .child(
                                            Label::new("No task has run yet").color(Color::Muted),
                                        )
                                        .child(
                                            Button::new(
                                                "zedx-choose-task",
                                                "Choose run configuration",
                                            )
                                            .on_click(
                                                |_, window, cx| {
                                                    window.dispatch_action(
                                                        zed_actions::Spawn::modal().boxed_clone(),
                                                        cx,
                                                    )
                                                },
                                            ),
                                        ),
                                )
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn task_selection_follows_new_tasks_preserves_manual_choice_and_handles_removal(
        cx: &mut gpui::TestAppContext,
    ) {
        let first = cx.new(|_| 0u8).entity_id();
        let second = cx.new(|_| 1u8).entity_id();
        assert_eq!(selected_task(None, &[], &[first]), Some(first));
        assert_eq!(
            selected_task(Some(first), &[first], &[first, second]),
            Some(second)
        );
        assert_eq!(
            selected_task(Some(first), &[first, second], &[first, second]),
            Some(first)
        );
        assert_eq!(
            selected_task(Some(first), &[first, second], &[second]),
            Some(second)
        );
        assert_eq!(selected_task(Some(second), &[second], &[]), None);
    }
}

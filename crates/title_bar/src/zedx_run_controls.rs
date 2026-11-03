use super::*;
use gpui::EntityId;
use terminal::{TaskStatus, Terminal};
use terminal_view::{TerminalView, terminal_panel::TerminalPanel};

#[derive(Default)]
pub(super) struct RunObservers {
    sources: Vec<EntityId>,
    subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct RunningTask {
    terminal: WeakEntity<Terminal>,
    label: String,
}

fn task_panes(workspace: &Workspace, cx: &App) -> Vec<Entity<workspace::Pane>> {
    let mut panes = workspace.panes().to_vec();
    if let Some(panel) = workspace.panel::<TerminalPanel>(cx) {
        panes.extend(panel.read(cx).panes().into_iter().cloned());
    }
    panes
}

#[derive(Debug, PartialEq)]
enum StopMode {
    Hidden,
    Single,
    Choose,
}
fn stop_mode(count: usize) -> StopMode {
    match count {
        0 => StopMode::Hidden,
        1 => StopMode::Single,
        _ => StopMode::Choose,
    }
}
fn is_running(status: Option<TaskStatus>) -> bool {
    status == Some(TaskStatus::Running)
}

fn running_tasks(workspace: &WeakEntity<Workspace>, cx: &App) -> Vec<RunningTask> {
    let Some(workspace) = workspace.upgrade() else {
        return vec![];
    };
    task_panes(workspace.read(cx), cx)
        .into_iter()
        .flat_map(|pane| {
            pane.read(cx)
                .items_of_type::<TerminalView>()
                .collect::<Vec<_>>()
        })
        .filter_map(|view| {
            let terminal = view.read(cx).terminal();
            let task = terminal.read(cx).task()?;
            is_running(Some(task.status)).then(|| RunningTask {
                terminal: terminal.downgrade(),
                label: task.spawned_task.full_label.to_string(),
            })
        })
        .collect()
}

fn stop_task(terminal: &WeakEntity<Terminal>, cx: &mut App) {
    if let Some(terminal) = terminal.upgrade() {
        // The menu may outlive its task. Native termination rechecks the status.
        terminal.update(cx, |terminal, _| terminal.kill_active_task());
    }
}

impl TitleBar {
    pub(super) fn observe_zedx_run_tasks(&mut self, cx: &mut Context<Self>) {
        if !zedx_title_layout::enabled() {
            return;
        }
        let panel = self
            .workspace
            .upgrade()
            .and_then(|workspace| workspace.read(cx).panel::<TerminalPanel>(cx));
        let panes = self
            .workspace
            .upgrade()
            .map(|workspace| task_panes(workspace.read(cx), cx))
            .unwrap_or_default();
        let terminals = panes
            .iter()
            .flat_map(|pane| {
                pane.read(cx)
                    .items_of_type::<TerminalView>()
                    .map(|view| view.read(cx).terminal().clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let sources = panel
            .iter()
            .map(Entity::entity_id)
            .chain(panes.iter().map(Entity::entity_id))
            .chain(terminals.iter().map(Entity::entity_id))
            .collect::<Vec<_>>();
        if self.zedx_run_observers.sources == sources {
            return;
        }
        self.zedx_run_observers.sources = sources;
        let mut subscriptions = Vec::new();
        if let Some(panel) = panel {
            subscriptions.push(cx.observe(&panel, |_, _, cx| cx.notify()));
        }
        for pane in panes {
            subscriptions.push(cx.observe(&pane, |_, _, cx| cx.notify()));
        }
        for terminal in terminals {
            subscriptions.push(cx.observe(&terminal, |_, _, cx| cx.notify()));
        }
        self.zedx_run_observers.subscriptions = subscriptions;
    }

    pub(super) fn render_zedx_stop(&self, window: &Window, cx: &App) -> Option<AnyElement> {
        let tasks = running_tasks(&self.workspace, cx);
        let mode = stop_mode(tasks.len());
        if mode == StopMode::Hidden {
            return None;
        }
        let controls = zedx_title_layout::control_size(window);
        let label = if mode == StopMode::Single {
            format!("Stop {}", tasks[0].label)
        } else {
            format!("Stop… ({} running tasks)", tasks.len())
        };
        let trigger = ButtonLike::new("zedx-stop")
            .width(controls.width)
            .height(controls.height.into())
            .aria_label(label.clone())
            .tab_index(0isize)
            .tooltip(Tooltip::text(label))
            .child(
                div()
                    .size(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .size(px(16.0))
                            .rounded(px(2.0))
                            .bg(gpui::rgb(0xdb5c5c)),
                    ),
            );
        let control = if mode == StopMode::Single {
            let task = tasks[0].terminal.clone();
            trigger
                .on_click(move |_, _, cx| stop_task(&task, cx))
                .into_any_element()
        } else {
            let workspace = self.workspace.clone();
            PopoverMenu::new("zedx-stop-menu")
                .trigger(trigger)
                .menu(move |window, cx| {
                    let tasks = running_tasks(&workspace, cx);
                    Some(ContextMenu::build(window, cx, move |mut menu, _, _| {
                        for task in &tasks {
                            let terminal = task.terminal.clone();
                            menu =
                                menu.entry(format!("Stop {}", task.label), None, move |_, cx| {
                                    stop_task(&terminal, cx)
                                });
                        }
                        menu
                    }))
                })
                .into_any_element()
        };
        Some(
            div()
                .debug_selector(|| "zedx-stop-control".into())
                .child(control)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_control_is_hidden_when_idle_and_chooses_between_concurrent_tasks() {
        assert_eq!(stop_mode(0), StopMode::Hidden);
        assert_eq!(stop_mode(1), StopMode::Single);
        assert_eq!(stop_mode(2), StopMode::Choose);
        assert_eq!(stop_mode(5), StopMode::Choose);
    }

    #[test]
    fn shells_and_finished_tasks_never_offer_stop() {
        assert!(!is_running(None));
        assert!(!is_running(Some(TaskStatus::Unknown)));
        assert!(!is_running(Some(TaskStatus::Completed { success: true })));
        assert!(!is_running(Some(TaskStatus::Completed { success: false })));
        assert!(is_running(Some(TaskStatus::Running)));
    }
}

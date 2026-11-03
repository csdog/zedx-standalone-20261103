use super::*;
use anyhow::Context as _;
use gpui::WeakEntity;
use language::language_settings::LanguageSettings;
use ui::{ContextMenu, ContextMenuEntry, IconName};
use util::ResultExt;
use workspace::notifications::NotifyTaskExt;

#[path = "zedx_context_run_policy.rs"]
mod policy;

#[derive(Clone)]
struct Target {
    buffer: Entity<Buffer>,
    version: Global,
    row: u32,
    tasks: Arc<RunnableTasks>,
    label: String,
    debug: bool,
}

pub(crate) struct RunMenu {
    editor: WeakEntity<Editor>,
    targets: Vec<Target>,
    run_color: ui::Color,
}
impl RunMenu {
    pub(crate) fn append(self, mut menu: ContextMenu) -> ContextMenu {
        if std::env::var_os("ZEDX_UPSTREAM_UI").is_some() {
            return menu;
        }
        for debug in [false, true] {
            let label = if debug { "Debug" } else { "Run" };
            let icon = if debug {
                IconName::Debug
            } else {
                IconName::ZedxRun
            };
            let color = if debug {
                ui::Color::Default
            } else {
                self.run_color
            };
            if self.targets.len() > 1 {
                let targets = self.targets.clone();
                let editor = self.editor.clone();
                menu = menu.submenu_with_colored_icon(label, icon, color, move |mut menu, _, _| {
                    for target in &targets {
                        menu = menu.item(Self::entry(
                            &target.label,
                            icon,
                            color,
                            Some(target.clone()),
                            debug,
                            editor.clone(),
                        ));
                    }
                    menu
                });
            } else {
                menu = menu.item(Self::entry(
                    label,
                    icon,
                    color,
                    self.targets.first().cloned(),
                    debug,
                    self.editor.clone(),
                ));
            }
        }
        menu.separator()
    }
    fn entry(
        label: &str,
        icon: IconName,
        color: ui::Color,
        target: Option<Target>,
        debug: bool,
        editor: WeakEntity<Editor>,
    ) -> ContextMenuEntry {
        ContextMenuEntry::new(label.to_string())
            .icon(icon)
            .icon_color(color)
            .disabled(target.as_ref().is_none_or(|target| debug && !target.debug))
            .handler(move |window, cx| {
                if let Some(target) = target.clone() {
                    editor
                        .update(cx, |editor, cx| {
                            if let Some((workspace, _)) = editor.workspace.clone() {
                                editor
                                    .launch_zedx_context_target(target, debug, window, cx)
                                    .detach_and_notify_err(workspace, window, cx);
                            }
                        })
                        .log_err();
                }
            })
    }
}

impl Editor {
    pub(crate) fn zedx_run_menu(&self, cx: &Context<Self>) -> RunMenu {
        let mut menu = RunMenu {
            editor: cx.weak_entity(),
            targets: vec![],
            run_color: ui::Color::Custom(
                gpui::rgb(if cx.theme().appearance().is_light() {
                    0x208a3c
                } else {
                    0x57965c
                })
                .into(),
            ),
        };
        if self.workspace().is_none() || self.project.is_none() {
            return menu;
        }
        let snapshot = self.buffer.read(cx).snapshot(cx);
        let selection = self.selections.newest_anchor();
        let Some((start, buffer_snapshot)) = snapshot.anchor_to_buffer_anchor(selection.start)
        else {
            return menu;
        };
        let Some((end, end_buffer)) = snapshot.anchor_to_buffer_anchor(selection.end) else {
            return menu;
        };
        if buffer_snapshot.remote_id() != end_buffer.remote_id() {
            return menu;
        }
        let buffer_id = buffer_snapshot.remote_id();
        let Some((version, tasks)) = self.runnables.runnables.get(&buffer_id) else {
            return menu;
        };
        let Some(buffer) = self.buffer.read(cx).buffer(buffer_id) else {
            return menu;
        };
        if buffer.read(cx).version() != *version {
            return menu;
        }
        let ranges = tasks
            .iter()
            .filter(|(_, tasks)| !tasks.templates.is_empty())
            .map(|(row, tasks)| (*row, tasks.context_range.start.0..tasks.context_range.end.0))
            .collect::<Vec<_>>();
        let rows = policy::matching_rows(
            start.to_offset(buffer_snapshot)..end.to_offset(end_buffer),
            &ranges,
        );
        let debuggable = self
            .workspace()
            .is_some_and(|workspace| workspace.read(cx).debugger_provider().is_some())
            && buffer.read(cx).language().is_some_and(|language| {
                !LanguageSettings::for_buffer(buffer.read(cx), cx)
                    .debuggers
                    .is_empty()
                    || !language.config().debuggers.is_empty()
            });
        for row in rows {
            let Some(tasks) = tasks.get(&row) else {
                continue;
            };
            let source = buffer_snapshot
                .chars_for_range(tasks.context_range.start.0..tasks.context_range.end.0)
                .take_while(|ch| *ch != '\n')
                .take(80)
                .collect::<String>();
            menu.targets.push(Target {
                buffer: buffer.clone(),
                version: version.clone(),
                row,
                tasks: Arc::new(tasks.clone()),
                label: format!("{} · line {}", source.trim(), row + 1),
                debug: debuggable,
            });
        }
        menu
    }

    fn launch_zedx_context_target(
        &mut self,
        target: Target,
        debug: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        if target.buffer.read(cx).version() != target.version {
            return Task::ready(Err(anyhow::anyhow!(
                "Code changed. Reopen the Run/Debug menu to choose the current target."
            )));
        }
        let Some(project) = self.project.clone() else {
            return Task::ready(Err(anyhow::anyhow!("No project for this runnable.")));
        };
        let Some(workspace) = self.workspace() else {
            return Task::ready(Err(anyhow::anyhow!("Workspace closed.")));
        };
        let context =
            Self::build_tasks_context(&project, &target.buffer, target.row, &target.tasks, cx);
        cx.spawn_in(window, async move |editor, cx| {
            let context = context
                .await?
                .context("No task context for the selected code")?;
            let resolved_tasks = target.tasks.resolve(&context).collect::<Vec<_>>();
            anyhow::ensure!(
                !resolved_tasks.is_empty(),
                "No runnable task can be resolved for the selected code"
            );
            // Task context resolution is asynchronous; validate the captured source again before launch.
            let current_version = target.buffer.read_with(cx, |buffer, _| buffer.version());
            anyhow::ensure!(
                current_version == target.version,
                "Code changed. Reopen the Run/Debug menu."
            );
            if debug {
                anyhow::ensure!(
                    target.debug,
                    "No debugger is configured for the selected code"
                );
                let tasks = Some(ResolvedTasks {
                    templates: resolved_tasks.into_iter().collect(),
                    position: target.tasks.offset,
                });
                let scenarios = editor
                    .update(cx, |editor, cx| {
                        editor.debug_scenarios(&tasks, &target.buffer, cx)
                    })?
                    .await;
                anyhow::ensure!(
                    target.buffer.read_with(cx, |buffer, _| buffer.version()) == target.version,
                    "Code changed. Reopen the Run/Debug menu."
                );
                let scenario = scenarios
                    .into_iter()
                    .next()
                    .context("The configured debugger cannot debug this runnable")?;
                workspace.update_in(cx, |workspace, window, cx| -> anyhow::Result<()> {
                    anyhow::ensure!(
                        workspace.debugger_provider().is_some(),
                        "Debugger is unavailable in this workspace"
                    );
                    workspace.start_debug_session(
                        scenario,
                        context.into(),
                        Some(target.buffer),
                        None,
                        window,
                        cx,
                    );
                    Ok(())
                })??;
            } else {
                let (kind, resolved) = resolved_tasks
                    .into_iter()
                    .next()
                    .context("No runnable task for the selected code")?;
                let buffer_id = target.buffer.read_with(cx, |buffer, _| buffer.remote_id());
                editor.update(cx, |editor, cx| {
                    editor.set_runnable_task_status(
                        buffer_id,
                        target.row,
                        RunnableTaskStatus::Running,
                        cx,
                    )
                })?;
                workspace.update_in(cx, |workspace, window, cx| {
                    workspace.schedule_resolved_task_with_completion(
                        kind,
                        resolved,
                        false,
                        move |result, cx| {
                            editor
                                .update(cx, |editor, cx| {
                                    editor.set_runnable_task_status(
                                        buffer_id,
                                        target.row,
                                        RunnableTaskStatus::from(result),
                                        cx,
                                    )
                                })
                                .log_err();
                        },
                        window,
                        cx,
                    );
                })?;
            }
            Ok(())
        })
    }
}

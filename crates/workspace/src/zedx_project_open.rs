use super::*;
use crate::{OpenOptions, OpenVisible, WorkspaceMatching};

#[derive(Clone, Copy)]
enum Choice {
    ThisWindow,
    NewWindow,
    Attach,
}

impl MultiWorkspace {
    pub fn open_project_with_zedx_prompt(
        &mut self,
        paths: Vec<PathBuf>,
        create_new_window: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let workspace = self.workspace().clone();
        let app_state = workspace.read(cx).app_state().clone();
        let group = workspace.read(cx).project_group_key(cx);
        let closing: Vec<_> = self
            .workspaces()
            .filter(|workspace| workspace.read(cx).project_group_key(cx) == group)
            .cloned()
            .collect();
        let can_attach = workspace.read(cx).project().read(cx).is_local();
        cx.spawn_in(window, async move |this, cx| {
            if paths.is_empty() {
                return Ok(());
            }
            let mut has_directory = false;
            for path in &paths {
                let metadata = app_state
                    .fs
                    .metadata(path)
                    .await?
                    .with_context(|| format!("{} does not exist", path.display()))?;
                has_directory |= metadata.is_dir;
            }
            let choice = if has_directory {
                let details = paths
                    .iter()
                    .map(|path| path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("\n");
                let prompt = this.update_in(cx, |_, window, cx| {
                    window.prompt(
                        gpui::PromptLevel::Info,
                        "Open Project",
                        Some(&format!(
                            "Where would you like to open this project?\n\n{details}"
                        )),
                        if can_attach {
                            &["This Window", "New Window", "Attach", "Cancel"]
                        } else {
                            &["This Window", "New Window", "Cancel"]
                        },
                        cx,
                    )
                })?;
                match prompt.await? {
                    0 => Choice::ThisWindow,
                    1 => Choice::NewWindow,
                    2 if can_attach => Choice::Attach,
                    _ => return Ok(()),
                }
            } else if create_new_window || !can_attach {
                Choice::NewWindow
            } else {
                Choice::Attach
            };
            match choice {
                Choice::Attach => {
                    anyhow::ensure!(
                        can_attach,
                        "Local folders cannot be attached to a remote project"
                    );
                    let results = workspace
                        .update_in(cx, |workspace, window, cx| {
                            workspace.open_paths(
                                paths,
                                OpenOptions {
                                    visible: Some(OpenVisible::All),
                                    ..Default::default()
                                },
                                None,
                                window,
                                cx,
                            )
                        })?
                        .await;
                    for result in results.into_iter().flatten() {
                        result?;
                    }
                }
                Choice::NewWindow => {
                    cx.update(|_, cx| {
                        crate::open_paths(
                            &paths,
                            app_state,
                            OpenOptions {
                                open_mode: OpenMode::NewWindow,
                                workspace_matching: WorkspaceMatching::None,
                                ..Default::default()
                            },
                            cx,
                        )
                    })?
                    .await?;
                }
                Choice::ThisWindow => {
                    if group.host().is_none() && group.path_list() == &PathList::new(&paths) {
                        this.update_in(cx, |this, window, cx| {
                            this.activate(workspace.clone(), None, window, cx);
                        })?;
                        return Ok(());
                    }
                    // Obtain consent before loading a replacement. Keep the old project
                    // retained until loading succeeds, so an error cannot discard it.
                    for workspace in &closing {
                        if !workspace
                            .update_in(cx, |workspace, window, cx| {
                                workspace.prepare_to_close(CloseIntent::ReplaceWindow, window, cx)
                            })?
                            .await?
                        {
                            return Ok(());
                        }
                    }
                    let replacement = this
                        .update_in(cx, |this, window, cx| {
                            this.find_or_create_local_workspace(
                                PathList::new(&paths),
                                None,
                                None,
                                OpenMode::Activate,
                                None,
                                window,
                                cx,
                            )
                        })?
                        .await?;
                    this.update(cx, |this, cx| {
                        for workspace in closing {
                            if workspace != replacement && this.is_workspace_retained(&workspace) {
                                this.detach_workspace(&workspace, cx);
                            }
                        }
                        if this.workspaces_for_project_group(&group, cx).is_empty() {
                            this.project_groups.retain(|entry| entry.key != group);
                            cx.emit(MultiWorkspaceEvent::ProjectGroupsChanged);
                        }
                        this.serialize(cx);
                        cx.notify();
                    })?;
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
#[path = "zedx_project_open_tests.rs"]
mod tests;

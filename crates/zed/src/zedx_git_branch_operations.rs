use super::*;

impl GitHistory {
    pub(super) fn branch_writable(&self, cx: &App) -> bool {
        self.workspace
            .upgrade()
            .is_some_and(|workspace| !workspace.read(cx).project().read(cx).is_read_only(cx))
    }

    pub(super) fn begin_new_branch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.branches.operating
            || !self.branch_writable(cx)
            || self.git.read(cx).active_repository().is_none()
        {
            return;
        }
        let name = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("New branch name", window, cx);
            editor
        });
        window.focus(&name.focus_handle(cx), cx);
        self.branches.new_name = Some(name);
        self.branches.error = None;
        cx.notify();
    }

    pub(super) fn create_branch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.branches.operating || !self.branch_writable(cx) {
            return;
        }
        let Some(repository) = self.git.read(cx).active_repository() else {
            return;
        };
        let Some(editor) = &self.branches.new_name else {
            return;
        };
        let name = editor.read(cx).text(cx).trim().to_string();
        if name.is_empty() || name.starts_with('-') {
            self.branches.error = Some("Enter a valid branch name.".into());
            cx.notify();
            return;
        }
        let repository_id = repository.read(cx).id;
        let base = self.branches.selected.as_ref().map(ToString::to_string);
        let confirmation = window.prompt(
            gpui::PromptLevel::Info,
            "Create and check out branch?",
            Some(&format!(
                "Create {name} from {} and switch to it.",
                base.as_deref().unwrap_or("HEAD")
            )),
            &["Create and Checkout", "Cancel"],
            cx,
        );
        self.branches.operating = true;
        cx.notify();
        cx.spawn_in(window, async move |history, cx| {
            let result: anyhow::Result<bool> = async {
                if confirmation.await? != 0 {
                    return Ok(false);
                }
                let request = history.update(cx, |history, cx| {
                    anyhow::ensure!(history.branch_writable(cx), "This workspace is read only.");
                    anyhow::ensure!(
                        history
                            .git
                            .read(cx)
                            .active_repository()
                            .is_some_and(|active| active.entity_id() == repository.entity_id()),
                        "The active repository changed."
                    );
                    Ok(repository.update(cx, |repository, _| repository.create_branch(name, base)))
                })??;
                request.await??;
                Ok(true)
            }
            .await;
            history
                .update(cx, |history, cx| {
                    if history.repository_id != Some(repository_id) {
                        return;
                    }
                    history.branches.operating = false;
                    match result {
                        Ok(true) => {
                            history.branches.new_name = None;
                            history.reload_branches(cx);
                        }
                        Ok(false) => {}
                        Err(error) => history.branches.error = Some(error.to_string()),
                    }
                    cx.notify();
                })
                .log_err();
        })
        .detach();
    }

    pub(super) fn delete_branch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.branches.operating || !self.branch_writable(cx) {
            return;
        }
        let Some(entry) = self
            .branches
            .refs
            .iter()
            .find(|entry| Some(&entry.name) == self.branches.selected.as_ref())
            .cloned()
        else {
            return;
        };
        if entry.current || entry.group == "Tags" {
            return;
        }
        let Some(repository) = self.git.read(cx).active_repository() else {
            return;
        };
        let repository_id = repository.read(cx).id;
        let remote = entry.group == "Remote";
        let confirmation = window.prompt(
            gpui::PromptLevel::Warning,
            if remote {
                "Delete local remote-tracking reference?"
            } else {
                "Delete branch?"
            },
            Some(&format!(
                "{}{}",
                entry.label,
                if remote {
                    "\nThe branch on the remote server will remain."
                } else {
                    "\nUnmerged branches will not be force deleted."
                }
            )),
            &["Delete", "Cancel"],
            cx,
        );
        self.branches.operating = true;
        cx.notify();
        cx.spawn_in(window, async move |history, cx| {
            let result: anyhow::Result<bool> = async {
                if confirmation.await? != 0 {
                    return Ok(false);
                }
                let request = history.update(cx, |history, cx| {
                    anyhow::ensure!(history.branch_writable(cx), "This workspace is read only.");
                    anyhow::ensure!(
                        history
                            .git
                            .read(cx)
                            .active_repository()
                            .is_some_and(|active| active.entity_id() == repository.entity_id()),
                        "The active repository changed."
                    );
                    anyhow::ensure!(
                        !repository
                            .read(cx)
                            .branch
                            .as_ref()
                            .is_some_and(|branch| branch.ref_name == entry.name),
                        "Cannot delete the current branch."
                    );
                    Ok(repository.update(cx, |repository, _| {
                        repository.delete_branch(remote, entry.label.to_string(), false)
                    }))
                })??;
                request.await??;
                Ok(true)
            }
            .await;
            history
                .update(cx, |history, cx| {
                    if history.repository_id != Some(repository_id) {
                        return;
                    }
                    history.branches.operating = false;
                    match result {
                        Ok(true) => {
                            history.clear_reference_filter(cx);
                            history.reload_branches(cx);
                        }
                        Ok(false) => {}
                        Err(error) => history.branches.error = Some(error.to_string()),
                    }
                    cx.notify();
                })
                .log_err();
        })
        .detach();
    }
}

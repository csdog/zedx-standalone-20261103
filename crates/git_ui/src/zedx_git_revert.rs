use super::*;
use anyhow::Context as _;

impl GitGraph {
    pub(super) fn revert_zedx_commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.zedx_revert_pending {
            return;
        }
        let Some((sha, parents)) = self
            .selected_entry_idx
            .and_then(|index| self.graph_data.commits.get(index))
            .map(|commit| (commit.data.sha, commit.data.parents.clone()))
        else {
            return;
        };
        let Some(repository) = self.get_repository(cx) else {
            return;
        };
        let choices = revert_choices(&parents);
        let labels = choices.iter().map(String::as_str).collect::<Vec<_>>();
        let confirmation = window.prompt(gpui::PromptLevel::Warning,
            if parents.len() > 1 { "Revert merge commit: choose mainline" } else { "Revert this commit?" },
            Some(&format!("A new commit will undo {}. {}The working tree must be clean.", sha.display_short(),
                if parents.len() > 1 { "Choose the parent to keep; changes introduced by this merge relative to that parent will be undone. " } else { "" })),
            &labels, cx);
        self.zedx_revert_pending = true;
        cx.notify();
        cx.spawn_in(window, async move |graph, cx| {
            let result: anyhow::Result<bool> = async {
                let choice = confirmation.await?;
                let Some(mainline) = selected_mainline(parents.len(), choice) else {
                    return Ok(false);
                };
                let request = graph.update(cx, |graph, cx| {
                    let workspace = graph
                        .workspace
                        .upgrade()
                        .context("The workspace is closed")?;
                    let workspace = workspace.read(cx);
                    let project = workspace.project().read(cx);
                    anyhow::ensure!(!project.is_read_only(cx), "The workspace is read only");
                    anyhow::ensure!(
                        project
                            .active_repository(cx)
                            .is_some_and(|active| active.entity_id() == repository.entity_id()),
                        "The active repository changed"
                    );
                    anyhow::ensure!(
                        !workspace.items(cx).any(|item| item.is_dirty(cx)),
                        "Save open files before reverting a commit"
                    );
                    Ok(repository.update(cx, |repository, _| {
                        repository.zedx_revert_commit(sha, mainline)
                    }))
                })??;
                request.await??;
                Ok(true)
            }
            .await;
            graph
                .update(cx, |graph, cx| {
                    graph.zedx_revert_pending = false;
                    match result {
                        Ok(true) => graph.invalidate_state(cx),
                        Ok(false) => {}
                        Err(error) => {
                            graph
                                .workspace
                                .update(cx, |workspace, cx| {
                                    workspace.show_toast(
                                        workspace::Toast::new(
                                            workspace::notifications::NotificationId::named(
                                                "zedx-revert-error".into(),
                                            ),
                                            format!("Could not revert commit: {error}"),
                                        ),
                                        cx,
                                    );
                                })
                                .log_err();
                        }
                    }
                    cx.notify();
                })
                .log_err();
        })
        .detach();
    }
}

fn revert_choices(parents: &[Oid]) -> Vec<String> {
    let mut choices = if parents.len() > 1 {
        parents
            .iter()
            .enumerate()
            .map(|(index, parent)| {
                format!(
                    "Revert relative to Parent {} ({})",
                    index + 1,
                    parent.display_short()
                )
            })
            .collect()
    } else {
        vec!["Revert Commit".into()]
    };
    choices.push("Cancel".into());
    choices
}
fn selected_mainline(parent_count: usize, choice: usize) -> Option<Option<usize>> {
    if parent_count <= 1 {
        (choice == 0).then_some(None)
    } else {
        (choice < parent_count).then(|| Some(choice + 1))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancel_never_becomes_a_mainline_parent() {
        assert_eq!(selected_mainline(0, 0), Some(None));
        assert_eq!(selected_mainline(1, 1), None);
        assert_eq!(selected_mainline(2, 0), Some(Some(1)));
        assert_eq!(selected_mainline(2, 1), Some(Some(2)));
        assert_eq!(selected_mainline(2, 2), None);
        assert_eq!(selected_mainline(4, 4), None);
        assert_eq!(selected_mainline(2, usize::MAX), None);
    }
    #[test]
    fn merge_buttons_identify_the_actual_parent_commits() {
        let parents = [
            Oid::try_from("1111111111111111111111111111111111111111").unwrap(),
            Oid::try_from("2222222222222222222222222222222222222222").unwrap(),
        ];
        let choices = revert_choices(&parents);
        assert!(choices[0].contains("Parent 1 (1111111"));
        assert!(choices[1].contains("Parent 2 (2222222"));
        assert_eq!(choices[2], "Cancel");
    }
}

use super::*;

impl ProjectPanel {
    // Scratch rows use a hidden worktree. Keep native file actions pointed at that
    // entry rather than falling back to the last visible project root.
    pub(super) fn zedx_scratch_entry(&self, target: &Target, cx: &App) -> Option<SelectedEntry> {
        if !self.is_zedx_scratch_target(target, cx) {
            return None;
        }
        let (worktree, prefix) = self.zedx_groups.scratch.as_ref()?;
        let worktree = worktree.read(cx);
        let path = match target {
            Target::File(path) | Target::Folder(path, _) => path.path.as_ref(),
            Target::ScratchGroup | Target::Scratches => prefix.as_ref(),
            _ => return None,
        };
        Some(SelectedEntry {
            worktree_id: worktree.id(),
            entry_id: worktree.entry_for_path(path)?.id,
        })
    }

    pub(super) fn has_zedx_scratch_selection(&self, cx: &App) -> bool {
        self.selection.is_some_and(|selection| {
            self.zedx_groups
                .scratch
                .as_ref()
                .is_some_and(|(worktree, prefix)| {
                    let worktree = worktree.read(cx);
                    selection.worktree_id == worktree.id()
                        && worktree
                            .entry_for_id(selection.entry_id)
                            .is_some_and(|entry| entry.path.starts_with(prefix))
                })
        })
    }

    pub(super) fn is_zedx_scratch_target(&self, target: &Target, cx: &App) -> bool {
        match target {
            Target::ScratchGroup | Target::Scratches => true,
            Target::Folder(path, _) | Target::File(path) => self
                .zedx_groups
                .scratch
                .as_ref()
                .is_some_and(|(worktree, prefix)| {
                    path.worktree_id == worktree.read(cx).id() && path.path.starts_with(prefix)
                }),
            _ => false,
        }
    }

    pub(crate) fn open_zedx_scratch_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !enabled() || (self.selection.is_some() && !self.has_zedx_scratch_selection(cx)) {
            return false;
        }
        let Some(row) = self
            .zedx_groups
            .rows
            .iter()
            .find(|row| Some(&row.key) == self.zedx_groups.selected.as_ref())
        else {
            return false;
        };
        if !self.is_zedx_scratch_target(&row.target, cx) {
            return false;
        }
        let target = row.target.clone();
        let file_target = matches!(target, Target::File(_) | Target::Folder(_, _));
        self.selection = self.zedx_scratch_entry(&target, cx);
        let pasteable = self.selection.is_some() && self.has_pasteable_content(cx);
        let panel = cx.weak_entity();
        let disabled = self.zedx_groups.scratch_loading || !self.project.read(cx).is_local();
        let focus = self.focus_handle.clone();
        let menu = ContextMenu::build(window, cx, move |menu, _, _| {
            menu.zedx_scope("project")
                .context(focus)
                .item(
                    ui::ContextMenuEntry::new("New File")
                        .icon(ui::IconName::File)
                        .disabled(disabled)
                        .handler(move |window, cx| {
                            panel
                                .update(cx, |panel, cx| {
                                    panel
                                        .zedx_groups
                                        .expanded
                                        .insert("scratches-and-consoles".into());
                                    panel.zedx_groups.expanded.insert("scratches".into());
                                    panel.load_zedx_scratches(true, window, cx);
                                })
                                .log_err();
                        }),
                )
                .separator()
                .when(file_target, |menu| {
                    menu.action("Cut", Box::new(Cut))
                        .action("Copy", Box::new(Copy))
                        .action("Duplicate", Box::new(Duplicate))
                })
                .action_disabled_when(!pasteable, "Paste", Box::new(Paste))
                .when(file_target, |menu| {
                    menu.separator()
                        .action("Rename", Box::new(Rename))
                        .action("Copy Path", Box::new(zed_actions::workspace::CopyPath))
                        .action(
                            "Copy Relative Path",
                            Box::new(zed_actions::workspace::CopyRelativePath),
                        )
                        .separator()
                        .action(
                            ui::utils::reveal_in_file_manager_label(false),
                            Box::new(RevealInFileManager),
                        )
                        .action("Open in Terminal", Box::new(OpenInTerminal))
                        .separator()
                        .action("Trash", Box::new(Trash { skip_prompt: false }))
                        .action("Delete", Box::new(Delete { skip_prompt: false }))
                })
        });
        window.focus(&menu.focus_handle(cx), cx);
        let subscription = cx.subscribe(&menu, |this, _, _: &DismissEvent, cx| {
            this.context_menu.take();
            cx.notify();
        });
        self.context_menu = Some(DeployedContextMenu {
            menu,
            position,
            anchor: None,
            _subscription: subscription,
        });
        cx.notify();
        true
    }
}

use super::*;

impl GitGraph {
    pub(super) fn render_zedx_commit_details(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(commit) = self
            .selected_entry_idx
            .and_then(|index| self.graph_data.commits.get(index))
        else {
            return Empty.into_any_element();
        };
        let Some(repository) = self.get_repository(cx) else {
            return Empty.into_any_element();
        };
        let sha: SharedString = commit.data.sha.to_string().into();
        let data = repository.update(cx, |repository, cx| {
            repository
                .fetch_commit_data(commit.data.sha, false, cx)
                .clone()
        });
        let (subject, metadata) = match data {
            CommitDataState::Loaded(data) => (
                data.subject.clone(),
                format!(
                    "{} · {} · {}",
                    data.author_name,
                    format_timestamp(data.commit_timestamp),
                    commit.data.sha.display_short()
                ),
            ),
            CommitDataState::Loading(_) => ("Loading…".into(), commit.data.sha.display_short()),
        };
        let (count, tree) = self.zedx_details_cache.borrow_mut().tree(
            commit.data.sha,
            self.selected_commit_diff.as_ref(),
            &self.changed_files_expanded_dirs,
            cx,
        );
        let tree_count = tree.len();
        let graph = cx.weak_entity();
        let workspace = self.workspace.clone();
        let repository = repository.downgrade();
        let message = self
            .selected_commit_message
            .as_ref()
            .filter(|message| message.sha == commit.data.sha);
        let message_style = editor::hover_markdown_style(window, cx);
        v_flex()
            .debug_selector(|| "zedx-git-details".into())
            .min_w(px(210.0))
            .h_full()
            .min_h_0()
            .flex_basis(DefiniteLength::Fraction(
                self.commit_details_split_state.read(cx).right_ratio(),
            ))
            .bg(cx.theme().colors().editor_background)
            .child(
                v_flex()
                    .debug_selector(|| "zedx-git-details-files".into())
                    .flex_1()
                    .flex_basis(DefiniteLength::Fraction(0.58))
                    .min_h_0()
                    .child(
                        h_flex()
                            .h(px(29.0))
                            .flex_none()
                            .px_1()
                            .gap_1()
                            .child(
                                IconButton::new("zedx-git-show-diff", IconName::Split)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Show Diff"))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_selected_commit_view(window, cx)
                                    })),
                            )
                            .child(
                                IconButton::new("zedx-git-revert", IconName::RotateCcw)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Revert Commit…"))
                                    .disabled(self.zedx_revert_pending)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.revert_zedx_commit(window, cx)
                                    })),
                            )
                            .child(
                                Label::new(if self.selected_commit_diff.is_some() {
                                    format!("{count} {}", if count == 1 { "file" } else { "files" })
                                } else {
                                    "Loading files…".into()
                                })
                                .size(ui::zedx::caption_size())
                                .color(Color::Muted),
                            )
                            .child(div().flex_1())
                            .child(
                                IconButton::new("zedx-git-expand-files", IconName::ChevronDown)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Expand All"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.changed_files_expanded_dirs.clear();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                IconButton::new("zedx-git-close-details", IconName::Close)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Close commit details"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.selected_entry_idx = None;
                                        this.selected_commit_diff = None;
                                        this.selected_commit_diff_stats = None;
                                        this.selected_commit_message = None;
                                        this._selected_commit_message_task = None;
                                        this._commit_diff_task = None;
                                        this.changed_files_expanded_dirs.clear();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(
                                uniform_list(
                                    "zedx-git-detail-files",
                                    tree_count,
                                    move |range, _, cx| {
                                        range
                                            .filter_map(|index| {
                                                tree.get(index).map(|entry| match entry {
                                                    ChangedFileTreeEntry::Directory(entry) => {
                                                        entry.render(index, graph.clone(), cx)
                                                    }
                                                    ChangedFileTreeEntry::File(entry) => {
                                                        entry.entry.render(
                                                            index,
                                                            entry.depth,
                                                            None,
                                                            sha.clone(),
                                                            repository.clone(),
                                                            workspace.clone(),
                                                            cx,
                                                        )
                                                    }
                                                })
                                            })
                                            .collect()
                                    },
                                )
                                .size_full()
                                .track_scroll(&self.changed_files_scroll_handle),
                            )
                            .vertical_scrollbar_for(&self.changed_files_scroll_handle, window, cx),
                    ),
            )
            .child(Divider::horizontal())
            .child(
                v_flex()
                    .debug_selector(|| "zedx-git-details-message".into())
                    .min_h_0()
                    .min_w_0()
                    .map(|section| {
                        if self.zedx_commit_info_collapsed {
                            section.flex_none()
                        } else {
                            section
                                .flex_1()
                                .flex_basis(DefiniteLength::Fraction(0.42))
                                .min_h(px(80.0))
                        }
                    })
                    .child(
                        h_flex()
                            .h(px(28.0))
                            .flex_none()
                            .px_1()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .debug_selector(|| "zedx-git-toggle-commit-info".into())
                                    .child(
                                        IconButton::new(
                                            "zedx-git-toggle-commit-info",
                                            if self.zedx_commit_info_collapsed {
                                                IconName::ChevronRight
                                            } else {
                                                IconName::ChevronDown
                                            },
                                        )
                                        .icon_size(IconSize::Small)
                                        .tab_index(0isize)
                                        .aria_label(if self.zedx_commit_info_collapsed {
                                            "Expand commit information"
                                        } else {
                                            "Collapse commit information"
                                        })
                                        .tooltip(Tooltip::text(
                                            if self.zedx_commit_info_collapsed {
                                                "Expand commit information"
                                            } else {
                                                "Collapse commit information"
                                            },
                                        ))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.zedx_commit_info_collapsed =
                                                    !this.zedx_commit_info_collapsed;
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            )
                            .child(
                                Label::new(subject.clone())
                                    .size(ui::zedx::label_size())
                                    .truncate(),
                            ),
                    )
                    .when(!self.zedx_commit_info_collapsed, |section| {
                        section.child(
                            v_flex()
                                .flex_1()
                                .min_h_0()
                                .min_w_0()
                                .px_2()
                                .pb_2()
                                .gap_1()
                                .child(
                                    div()
                                        .debug_selector(|| "zedx-git-commit-metadata".into())
                                        .child(
                                            Label::new(metadata)
                                                .size(ui::zedx::caption_size())
                                                .color(Color::Muted)
                                                .truncate(),
                                        ),
                                )
                                .child(div().flex_1().min_h_0().min_w_0().map(|element| {
                                    if let Some(message) = message {
                                        element
                                            .child(
                                                div()
                                                    .id("zedx-git-commit-message")
                                                    .debug_selector(|| {
                                                        "zedx-git-message-body".into()
                                                    })
                                                    .size_full()
                                                    .overflow_y_scroll()
                                                    .track_scroll(&message.scroll_handle)
                                                    .child(MarkdownElement::new(
                                                        message.message.clone(),
                                                        message_style,
                                                    )),
                                            )
                                            .vertical_scrollbar_for(
                                                &message.scroll_handle,
                                                window,
                                                cx,
                                            )
                                            .into_any_element()
                                    } else {
                                        element
                                            .child(
                                                div()
                                                    .debug_selector(|| {
                                                        "zedx-git-message-pending".into()
                                                    })
                                                    .child(
                                                        Label::new(subject)
                                                            .size(ui::zedx::label_size()),
                                                    ),
                                            )
                                            .into_any_element()
                                    }
                                })),
                        )
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs::FakeFs;
    use gpui::TestAppContext;
    use settings::SettingsStore;
    use std::path::Path;

    struct DetailsView(Entity<GitGraph>);

    impl Render for DetailsView {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            h_flex().size_full().child(
                self.0
                    .update(cx, |graph, cx| graph.render_zedx_commit_details(window, cx)),
            )
        }
    }

    #[gpui::test]
    async fn compact_details_keep_files_visible_and_reject_stale_messages(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings = SettingsStore::test(cx);
            cx.set_global(settings);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            language_model::init(cx);
            crate::init(cx);
        });
        let fs = FakeFs::new(cx.executor());
        fs.insert_tree(
            Path::new("/project"),
            serde_json::json!({".git": {}, "file.txt": "content"}),
        )
        .await;
        let sha = Oid::from_bytes(&[1; 20]).unwrap();
        fs.set_graph_commits(
            Path::new("/project/.git"),
            vec![Arc::new(InitialGraphCommitData {
                sha,
                parents: smallvec![],
                ref_names: vec!["HEAD -> main".into()],
            })],
        );
        let message = (0..40)
            .map(|index| format!("Long message paragraph {index}.\n\n"))
            .collect::<String>();
        fs.set_commit_data(
            Path::new("/project/.git"),
            [(
                git::repository::CommitData {
                    sha,
                    parents: smallvec![],
                    author_name: "Author".into(),
                    author_email: "author@example.invalid".into(),
                    commit_timestamp: 1_700_000_000,
                    subject: "Compact commit".into(),
                    message: message.into(),
                },
                false,
            )],
        );
        let project = project::Project::test(fs, [Path::new("/project")], cx).await;
        let repository =
            project.read_with(cx, |project, cx| project.active_repository(cx).unwrap());
        let (multi_workspace, cx) = cx.add_window_view(|window, cx| {
            workspace::MultiWorkspace::test_new(project.clone(), window, cx)
        });
        let workspace = multi_workspace.read_with(cx, |multi, _| multi.workspace().downgrade());
        let graph = cx.new_window_entity(|window, cx| {
            GitGraph::new(
                repository.read(cx).id,
                project.read(cx).git_store().clone(),
                workspace,
                None,
                window,
                cx,
            )
        });
        cx.run_until_parked();
        graph.update_in(cx, |graph, window, cx| {
            graph.use_zedx_toolbar(window, cx);
            graph.select_first(&SelectFirst, window, cx);
        });
        cx.run_until_parked();
        graph.update(cx, |graph, cx| {
            graph.selected_commit_diff = Some(CommitDiff {
                files: vec![CommitFile {
                    path: RepoPath::new("file.txt").unwrap(),
                    old_text: Some("content".into()),
                    new_text: Some("updated".into()),
                    is_binary: false,
                }],
                is_shallow_boundary: false,
            });
            cx.notify();
        });
        let details = cx.new_window_entity(|_, _| DetailsView(graph.clone()));
        for height in [202.0, 346.0] {
            cx.draw(
                point(px(0.0), px(0.0)),
                gpui::size(px(500.0), px(height)),
                |_, _| details.clone().into_any_element(),
            );
            let (files, message) = graph.read_with(cx, |graph, _| {
                (
                    graph
                        .changed_files_scroll_handle
                        .0
                        .borrow()
                        .base_handle
                        .bounds(),
                    graph
                        .selected_commit_message
                        .as_ref()
                        .unwrap()
                        .scroll_handle
                        .bounds(),
                )
            });
            assert!(
                files.size.height >= px(28.0),
                "files are visible at {height}px: {files:?}"
            );
            assert!(
                files.bottom() <= message.top(),
                "files {files:?} must precede message {message:?}"
            );
            assert!(message.size.height > px(0.0));
            assert!(
                message.bottom() <= px(height),
                "message must stay within the panel: {message:?}"
            );
            graph.read_with(cx, |graph, _| {
                assert!(
                    graph
                        .selected_commit_message
                        .as_ref()
                        .unwrap()
                        .scroll_handle
                        .max_offset()
                        .y
                        > px(0.0)
                )
            });
        }
        cx.update(|window, cx| window.replace_root(cx, |_, _| DetailsView(graph.clone())));
        cx.simulate_resize(gpui::size(px(500.0), px(202.0)));
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds("zedx-git-message-body").is_some());
        let files_before = graph.read_with(cx, |graph, _| {
            graph
                .changed_files_scroll_handle
                .0
                .borrow()
                .base_handle
                .bounds()
        });
        let toggle = cx.debug_bounds("zedx-git-toggle-commit-info").unwrap();
        cx.simulate_click(toggle.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(graph.read_with(cx, |graph, _| graph.zedx_commit_info_collapsed));
        assert!(cx.debug_bounds("zedx-git-commit-metadata").is_none());
        assert!(cx.debug_bounds("zedx-git-message-body").is_none());
        let files_after = graph.read_with(cx, |graph, _| {
            graph
                .changed_files_scroll_handle
                .0
                .borrow()
                .base_handle
                .bounds()
        });
        assert!(files_after.size.height > files_before.size.height);
        let toggle = cx.debug_bounds("zedx-git-toggle-commit-info").unwrap();
        cx.simulate_click(toggle.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(!graph.read_with(cx, |graph, _| graph.zedx_commit_info_collapsed));
        assert!(cx.debug_bounds("zedx-git-commit-metadata").is_some());
        assert!(cx.debug_bounds("zedx-git-message-body").is_some());
        graph.update(cx, |graph, cx| {
            graph.selected_commit_message.as_mut().unwrap().sha =
                Oid::from_bytes(&[2; 20]).unwrap();
            cx.notify();
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(
            cx.debug_bounds("zedx-git-message-body").is_none(),
            "a message belonging to the previous commit must never be rendered"
        );
        assert!(cx.debug_bounds("zedx-git-message-pending").is_some());
    }
}

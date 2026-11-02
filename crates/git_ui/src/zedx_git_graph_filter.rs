use super::*;

#[derive(Default)]
pub(super) struct FilterState {
    pub(super) projection: Option<Rc<zedx_filtered_graph::Projection>>,
    generation: u64,
    loading: bool,
    _editor_subscription: Option<Subscription>,
}

pub(super) enum Navigation {
    First,
    Last,
    Offset(isize),
}

impl GitGraph {
    pub(super) fn initialize_zedx_filter(&mut self, cx: &mut Context<Self>) {
        if self.zedx_filter._editor_subscription.is_some() {
            return;
        }
        self.zedx_filter._editor_subscription = Some(cx.subscribe(
            &self.search_state.editor,
            |this, editor, event, cx| {
                if matches!(event, editor::EditorEvent::BufferEdited) {
                    let query = editor.read(cx).text(cx).into();
                    this.search(query, cx);
                }
            },
        ));
    }

    pub(super) fn zedx_filter_active(&self) -> bool {
        self.zedx_toolbar.is_some()
            && match &self.search_state.state {
                QueryState::Pending(query) | QueryState::Confirmed((query, _)) => !query.is_empty(),
                QueryState::Empty => false,
            }
    }

    pub(super) fn zedx_filtered_indices(&self) -> Vec<usize> {
        let query = match &self.search_state.state {
            QueryState::Pending(query) | QueryState::Confirmed((query, _)) => {
                query.to_ascii_lowercase()
            }
            QueryState::Empty => return Vec::new(),
        };
        let search_hash = !query.is_empty() && query.bytes().all(|byte| byte.is_ascii_hexdigit());
        self.graph_data
            .commits
            .iter()
            .enumerate()
            .filter_map(|(index, commit)| {
                (self.search_state.matches.contains(&commit.data.sha)
                    || (search_hash && commit.data.sha.to_string().contains(&query)))
                .then_some(index)
            })
            .collect()
    }

    pub(super) fn zedx_filtered_row_index(&self, index: usize) -> Option<usize> {
        self.zedx_filter_active()
            .then(|| {
                self.zedx_filtered_indices()
                    .iter()
                    .position(|&row| row == index)
            })
            .flatten()
    }

    fn clear_zedx_commit_selection(&mut self) {
        self.selected_entry_idx = None;
        self.hovered_entry_idx = None;
        self.pending_select_sha = None;
        self.selected_commit_diff = None;
        self.selected_commit_diff_stats = None;
        self.selected_commit_message = None;
        self._commit_diff_task = None;
        self._selected_commit_message_task = None;
    }

    pub(super) fn search_zedx_commits(&mut self, query: SharedString, cx: &mut Context<Self>) {
        let query: SharedString = query.trim().into();
        self.zedx_filter.projection = None;
        self.zedx_filter.generation += 1;
        let generation = self.zedx_filter.generation;
        self.search_state.state = QueryState::Empty;
        self.search_state.matches.clear();
        self.search_state.selected_index = None;
        self.search_state.editor.update(cx, |editor, _| {
            editor.set_text_style_refinement(Default::default())
        });
        self.table_interaction_state.update(cx, |state, _| {
            state.scroll_handle.scroll_to_item(0, ScrollStrategy::Top)
        });
        self.zedx_filter.loading = false;
        if query.is_empty() {
            cx.notify();
            return;
        }
        self.clear_zedx_commit_selection();
        let Some(repository) = self.get_repository(cx) else {
            cx.notify();
            return;
        };
        let source = self.log_source.clone();
        let repo_id = self.repo_id;
        let (sender, receiver) = async_channel::unbounded();
        // The backend interprets a long hex-only query as a hash prefix. Search a
        // shorter text prefix and verify subjects so hex text and hash fragments both work.
        let text_query = if git::repository::commit_hash_search_query(&query).is_some() {
            SharedString::from(query.chars().take(6).collect::<String>())
        } else {
            query.clone()
        };
        let case_sensitive = self.search_state.case_sensitive;
        let subject_query = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        repository.update(cx, |repository, cx| {
            repository.search_commits(
                source.clone(),
                SearchCommitArgs {
                    query: text_query,
                    case_sensitive,
                },
                sender,
                cx,
            );
        });
        self.zedx_filter.loading = true;
        let task = cx.spawn(async move |this, cx| {
            while let Ok(first) = receiver.recv().await {
                let mut matches = vec![first];
                while let Ok(sha) = receiver.try_recv() {
                    matches.push(sha);
                }
                let candidates = repository.update(cx, |repository, cx| {
                    matches
                        .into_iter()
                        .map(|sha| (sha, repository.fetch_commit_data(sha, true, cx).clone()))
                        .collect::<Vec<_>>()
                });
                let mut matches = Vec::new();
                for (sha, state) in candidates {
                    let data = match state {
                        CommitDataState::Loaded(data) => Some(data),
                        CommitDataState::Loading(Some(receiver)) => receiver.await.ok(),
                        CommitDataState::Loading(None) => None,
                    };
                    if data.is_some_and(|data| {
                        if case_sensitive {
                            data.subject.contains(&subject_query)
                        } else {
                            data.subject.to_lowercase().contains(&subject_query)
                        }
                    }) {
                        matches.push(sha);
                    }
                }
                this.update(cx, |this, cx| {
                    if this.zedx_filter.generation != generation
                        || this.repo_id != repo_id
                        || this.log_source != source
                    {
                        return;
                    }
                    this.search_state.matches.extend(matches);
                    if this.selected_entry_idx.is_none() {
                        if let Some(index) = this.zedx_filtered_indices().first().copied() {
                            this.search_state.selected_index = Some(0);
                            this.select_entry(index, ScrollStrategy::Top, cx);
                        }
                    }
                    cx.notify();
                })
                .log_err();
            }
            this.update(cx, |this, cx| {
                if this.zedx_filter.generation == generation
                    && this.repo_id == repo_id
                    && this.log_source == source
                {
                    this.zedx_filter.loading = false;
                    if this.selected_entry_idx.is_none() {
                        if let Some(index) = this.zedx_filtered_indices().first().copied() {
                            this.select_entry(index, ScrollStrategy::Top, cx);
                        }
                    }
                    cx.notify();
                }
            })
            .log_err();
        });
        self.search_state.state = QueryState::Confirmed((query, task));
        cx.emit(ItemEvent::Edit);
        cx.notify();
    }

    pub(super) fn navigate_zedx_filter(
        &mut self,
        navigation: Navigation,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.zedx_filter_active() {
            return false;
        }
        let rows = self.zedx_filtered_indices();
        if rows.is_empty() {
            return true;
        }
        let selected = self
            .selected_entry_idx
            .and_then(|selected| rows.iter().position(|&row| row == selected));
        let position = match navigation {
            Navigation::First => 0,
            Navigation::Last => rows.len() - 1,
            Navigation::Offset(offset) => selected
                .map(|position| position.saturating_add_signed(offset).min(rows.len() - 1))
                .unwrap_or(0),
        };
        self.select_entry(rows[position], ScrollStrategy::Nearest, cx);
        true
    }

    fn render_zedx_filtered_rows(
        &mut self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let indices = self.zedx_filtered_indices();
        let fractions = self.preview_column_fractions(window, cx);
        let projection = self.zedx_graph_projection(&indices);
        range
            .filter_map(|position| {
                indices
                    .get(position)
                    .copied()
                    .map(|index| (position, index))
            })
            .map(|(position, index)| {
                let cells = self
                    .render_table_rows(index..index + 1, window, cx)
                    .pop()
                    .unwrap_or_default();
                let commit = &self.graph_data.commits[index];
                let primary_ref = commit
                    .data
                    .ref_names
                    .iter()
                    .find(|name| name.starts_with("HEAD"))
                    .or_else(|| commit.data.ref_names.first())
                    .cloned();
                let row_id = format!("zedx-git-filtered-{}", commit.data.sha);
                let graph_cell = h_flex()
                    .gap_1()
                    .overflow_hidden()
                    .child(projection.render_row(
                        position,
                        Self::row_height(window, cx),
                        if self.selected_entry_idx == Some(index) {
                            cx.theme().colors().element_selected
                        } else if self.hovered_entry_idx == Some(index) {
                            cx.theme().colors().element_hover
                        } else {
                            cx.theme().colors().editor_background
                        },
                    ))
                    .when_some(primary_ref, |row, name| {
                        let is_head = name.starts_with("HEAD");
                        row.child(
                            self.render_ref_chip(
                                name,
                                cx.theme()
                                    .accents()
                                    .color_for_index(commit.color_idx as u32),
                                is_head,
                                index,
                                cx.theme().colors().editor_background,
                                cx,
                            ),
                        )
                    })
                    .into_any_element();
                let cells = std::iter::once(graph_cell).chain(cells);
                h_flex()
                    .id(("zedx-filtered-commit", index))
                    .debug_selector(move || row_id)
                    .w_full()
                    .h(Self::row_height(window, cx))
                    .min_w_0()
                    .when(self.selected_entry_idx == Some(index), |row| {
                        row.bg(cx.theme().colors().element_selected)
                    })
                    .hover(|row| row.bg(cx.theme().colors().element_hover))
                    .on_hover(cx.listener(move |this, &hovered: &bool, _, cx| {
                        this.hovered_entry_idx = hovered.then_some(index);
                        cx.notify();
                    }))
                    .children(
                        cells
                            .zip(fractions)
                            .filter(|(_, fraction)| *fraction > 0.0)
                            .map(|(cell, fraction)| {
                                div()
                                    .w(DefiniteLength::Fraction(fraction))
                                    .min_w_0()
                                    .overflow_hidden()
                                    .px_1()
                                    .child(cell)
                            }),
                    )
                    .on_click(cx.listener(move |this, event, window, cx| {
                        let focus = this.focus_handle.clone();
                        this.handle_entry_click(
                            index,
                            event,
                            ScrollStrategy::Nearest,
                            Some(&focus),
                            window,
                            cx,
                        );
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                            this.handle_entry_secondary_mouse_down(index, event, window, cx);
                        }),
                    )
                    .into_any_element()
            })
            .collect()
    }

    pub(super) fn render_zedx_filtered_log(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let indices = self.zedx_filtered_indices();
        if self
            .selected_entry_idx
            .is_some_and(|index| !indices.contains(&index))
        {
            self.clear_zedx_commit_selection();
        }
        let fractions = self.preview_column_fractions(window, cx);
        let scroll_handle = self.table_interaction_state.read(cx).scroll_handle.clone();
        let list = v_flex()
            .relative()
            .size_full()
            .min_w_0()
            .child(
                h_flex()
                    .h(Self::row_height(window, cx))
                    .flex_none()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .children(
                        ["Graph", "Description", "Date", "Author", "Commit"]
                            .into_iter()
                            .zip(fractions)
                            .filter(|(_, fraction)| *fraction > 0.0)
                            .map(|(label, fraction)| {
                                div()
                                    .w(DefiniteLength::Fraction(fraction))
                                    .min_w_0()
                                    .overflow_hidden()
                                    .px_1()
                                    .child(Label::new(label).color(Color::Muted).truncate())
                            }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.deploy_header_context_menu(event.position, window, cx)
                        }),
                    ),
            )
            .child(div().flex_1().min_h_0().map(|element| {
                if indices.is_empty() {
                    element
                        .child(
                            div()
                                .debug_selector(|| "zedx-git-filter-empty".into())
                                .p_2()
                                .child(
                                    Label::new(if self.zedx_filter.loading {
                                        "Searching commits…"
                                    } else {
                                        "No matching commits"
                                    })
                                    .color(Color::Muted),
                                ),
                        )
                        .into_any_element()
                } else {
                    element
                        .child(
                            uniform_list(
                                "zedx-filtered-log",
                                indices.len(),
                                cx.processor(Self::render_zedx_filtered_rows),
                            )
                            .size_full()
                            .track_scroll(&scroll_handle),
                        )
                        .vertical_scrollbar_for(&scroll_handle, window, cx)
                        .into_any_element()
                }
            }))
            .child(render_redistributable_columns_resize_handles(
                &self.column_widths,
                Some(&self.column_visibility),
                window,
                cx,
            ));
        let list = bind_redistributable_columns(
            list,
            self.column_widths.clone(),
            Some(self.column_visibility.clone()),
        );
        h_flex()
            .size_full()
            .min_w_0()
            .child(div().flex_1().min_w_0().h_full().child(list))
            .on_drag_move::<DraggedSplitHandle>(cx.listener(|this, event, window, cx| {
                this.commit_details_split_state
                    .update(cx, |state, cx| state.on_drag_move(event, window, cx))
            }))
            .on_drop::<DraggedSplitHandle>(cx.listener(|this, _, _, cx| {
                this.commit_details_split_state
                    .update(cx, |state, _| state.commit_ratio())
            }))
            .when(
                self.selected_entry_idx.is_some() && window.viewport_size().width >= px(1100.0),
                |element| {
                    element
                        .child(self.render_commit_view_resize_handle(window, cx))
                        .child(self.render_zedx_commit_details(window, cx))
                },
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs::FakeFs;
    use gpui::{TestAppContext, VisualTestContext};
    use settings::SettingsStore;
    use std::path::Path;

    const FIRST: &str = "zedx-git-filtered-0101010101010101010101010101010101010101";
    const MIDDLE: &str = "zedx-git-filtered-0202020202020202020202020202020202020202";
    const LAST: &str = "zedx-git-filtered-0303030303030303030303030303030303030303";

    struct FilterView(Entity<GitGraph>);
    impl Render for FilterView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.0.clone()
        }
    }

    async fn setup(
        cx: &mut TestAppContext,
    ) -> (
        Entity<GitGraph>,
        Entity<workspace::MultiWorkspace>,
        &mut VisualTestContext,
    ) {
        setup_history(cx, 3).await
    }

    async fn setup_history(
        cx: &mut TestAppContext,
        count: usize,
    ) -> (
        Entity<GitGraph>,
        Entity<workspace::MultiWorkspace>,
        &mut VisualTestContext,
    ) {
        cx.update(|cx| {
            let settings = SettingsStore::test(cx);
            cx.set_global(settings);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            language_model::init(cx);
            crate::init(cx);
        });
        let fs = FakeFs::new(cx.executor());
        let path = Path::new("/project/.git");
        fs.insert_tree(
            Path::new("/project"),
            serde_json::json!({".git": {}, "file.txt": "content"}),
        )
        .await;
        let shas = (0..count)
            .map(|index| {
                let mut bytes = [(index + 1) as u8; 20];
                if count > 3 {
                    bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
                }
                Oid::from_bytes(&bytes).unwrap()
            })
            .collect::<Vec<_>>();
        fs.set_graph_commits(
            path,
            shas.iter()
                .enumerate()
                .map(|(index, &sha)| {
                    Arc::new(InitialGraphCommitData {
                        sha,
                        parents: shas.get(index + 1).copied().into_iter().collect(),
                        ref_names: Vec::new(),
                    })
                })
                .collect(),
        );
        fs.set_commit_data(
            path,
            shas.into_iter().enumerate().map(|(index, sha)| {
                let subject = if count == 3 {
                    ["Fix first deadbeef", "Unrelated", "Fix last"][index].to_string()
                } else {
                    format!(
                        "{} {index:05}",
                        if index % 2 == 0 { "Keep" } else { "Other" }
                    )
                };
                (
                    git::repository::CommitData {
                        sha,
                        parents: smallvec![],
                        author_name: "Author".into(),
                        author_email: "author@example.invalid".into(),
                        commit_timestamp: 1_700_000_000,
                        subject: subject.clone().into(),
                        message: if subject == "Unrelated" {
                            "Unrelated\n\nFix appears only in the body".into()
                        } else {
                            subject.into()
                        },
                    },
                    false,
                )
            }),
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
        graph.update_in(cx, |graph, window, cx| graph.use_zedx_toolbar(window, cx));
        cx.update(|window, cx| window.replace_root(cx, |_, _| FilterView(graph.clone())));
        cx.simulate_resize(gpui::size(px(1400.0), px(700.0)));
        cx.run_until_parked();
        cx.refresh().unwrap();
        (graph, multi_workspace, cx)
    }

    fn set_query(graph: &Entity<GitGraph>, query: &str, cx: &mut VisualTestContext) {
        graph.update_in(cx, |graph, window, cx| {
            graph
                .search_state
                .editor
                .update(cx, |editor, cx| editor.set_text(query, window, cx))
        });
    }

    #[gpui::test]
    async fn live_filter_keeps_original_selection_and_restores_the_graph(cx: &mut TestAppContext) {
        let (graph, _workspace, cx) = setup(cx).await;
        let query_bounds = cx.debug_bounds("zedx-git-log-query").unwrap();
        assert_eq!(query_bounds.size.width, px(170.0));
        assert_eq!(query_bounds.size.height, px(24.0));
        set_query(&graph, "  Fix  ", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds(FIRST).is_some());
        assert!(cx.debug_bounds(MIDDLE).is_none());
        assert!(cx.debug_bounds(LAST).is_some());
        graph.read_with(cx, |graph, _| {
            assert_eq!(graph.zedx_filtered_indices(), vec![0, 2]);
            assert_eq!(graph.graph_data.commits.len(), 3);
            assert_eq!(graph.selected_entry_idx, Some(0));
        });
        let last = cx.debug_bounds(LAST).unwrap();
        cx.simulate_click(last.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(2)));
        let first = cx.debug_bounds(FIRST).unwrap();
        cx.simulate_click(first.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(0)));
        graph.update_in(cx, |graph, window, cx| {
            graph.select_next(&SelectNext, window, cx)
        });
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(2)));
        graph.update_in(cx, |graph, window, cx| {
            graph.select_prev(&SelectPrevious, window, cx)
        });
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(0)));
        graph.update_in(cx, |graph, window, cx| {
            graph.select_last(&SelectLast, window, cx)
        });
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(2)));
        graph.update_in(cx, |graph, window, cx| {
            graph.select_first(&SelectFirst, window, cx)
        });
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(0)));
        set_query(&graph, " DEADBEEF ", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds(FIRST).is_some());
        assert!(cx.debug_bounds(LAST).is_none());
        assert!(cx.debug_bounds(MIDDLE).is_none());
        set_query(&graph, "1010", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds(FIRST).is_some());
        assert!(cx.debug_bounds(LAST).is_none());
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(0)));
        set_query(&graph, "0303030", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds(FIRST).is_none());
        assert!(cx.debug_bounds(LAST).is_some());
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(2)));
        set_query(&graph, "", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds(LAST).is_none());
        graph.read_with(cx, |graph, _| {
            assert!(!graph.zedx_filter_active());
            assert_eq!(graph.graph_data.commits.len(), 3);
            assert!(graph.search_state.matches.is_empty());
        });
    }

    #[gpui::test]
    async fn replacement_query_and_empty_results_never_keep_old_rows_or_details(
        cx: &mut TestAppContext,
    ) {
        let (graph, _workspace, cx) = setup(cx).await;
        set_query(&graph, "Fix", cx);
        set_query(&graph, "Unrelated", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds(FIRST).is_none());
        assert!(cx.debug_bounds(LAST).is_none());
        assert!(cx.debug_bounds(MIDDLE).is_some());
        graph.read_with(cx, |graph, _| assert_eq!(graph.selected_entry_idx, Some(1)));
        set_query(&graph, "does not exist", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds("zedx-git-filter-empty").is_some());
        assert!(cx.debug_bounds(MIDDLE).is_none());
        assert!(cx.debug_bounds("zedx-git-details").is_none());
        graph.update_in(cx, |graph, window, cx| {
            graph.select_next(&SelectNext, window, cx);
            graph.select_first(&SelectFirst, window, cx);
            graph.select_last(&SelectLast, window, cx);
        });
        graph.read_with(cx, |graph, _| {
            assert!(!graph.zedx_filter.loading);
            assert!(graph.selected_entry_idx.is_none());
            assert!(graph.selected_commit_message.is_none());
        });
        set_query(&graph, "Fix", cx);
        set_query(&graph, "", cx);
        cx.run_until_parked();
        cx.refresh().unwrap();
        graph.read_with(cx, |graph, _| {
            assert!(!graph.zedx_filter_active());
            assert!(graph.search_state.matches.is_empty());
            assert!(!graph.zedx_filter.loading);
        });
    }
    #[gpui::test]
    async fn long_filtered_history_scrolls_to_original_rows_and_replaces_queries(
        cx: &mut TestAppContext,
    ) {
        let (graph, _workspace, cx) = setup_history(cx, 4_000).await;
        set_query(&graph, "Keep", cx);
        cx.run_until_parked();
        graph.read_with(cx, |graph, _| {
            assert_eq!(graph.graph_data.commits.len(), 4_000);
            assert_eq!(graph.zedx_filtered_indices().len(), 2_000);
            assert!(!graph.zedx_filter.loading);
        });
        let first: &'static str = graph.read_with(cx, |graph, _| {
            format!("zedx-git-filtered-{}", graph.graph_data.commits[0].data.sha).leak()
        });
        let last: &'static str = graph.read_with(cx, |graph, _| {
            format!(
                "zedx-git-filtered-{}",
                graph.graph_data.commits[3_998].data.sha
            )
            .leak()
        });
        cx.refresh().unwrap();
        assert!(cx.debug_bounds(first).is_some());
        assert!(cx.debug_bounds(last).is_none());
        graph.update_in(cx, |graph, window, cx| {
            graph.select_last(&SelectLast, window, cx)
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        graph.read_with(cx, |graph, _| {
            assert_eq!(graph.selected_entry_idx, Some(3_998))
        });
        assert!(cx.debug_bounds(last).is_some());
        assert!(cx.debug_bounds(first).is_none());
        let row = cx.debug_bounds(last).unwrap();
        cx.simulate_click(row.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        graph.read_with(cx, |graph, _| {
            assert_eq!(graph.selected_entry_idx, Some(3_998))
        });
        set_query(&graph, "Keep", cx);
        set_query(&graph, "Other", cx);
        cx.run_until_parked();
        graph.read_with(cx, |graph, _| {
            assert_eq!(graph.zedx_filtered_indices().len(), 2_000);
            assert!(
                graph
                    .zedx_filtered_indices()
                    .iter()
                    .all(|index| index % 2 == 1)
            );
            assert_eq!(graph.selected_entry_idx, Some(1));
            assert!(!graph.zedx_filter.loading);
        });
        set_query(&graph, "Keep", cx);
        set_query(&graph, "", cx);
        cx.run_until_parked();
        graph.read_with(cx, |graph, _| {
            assert!(!graph.zedx_filter_active());
            assert!(graph.search_state.matches.is_empty());
            assert_eq!(graph.graph_data.commits.len(), 4_000);
        });
    }
}

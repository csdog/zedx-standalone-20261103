use super::*;
use git::repository::{LogOrder, LogSource};
use gpui::UniformListScrollHandle;

#[derive(Clone, Debug)]
struct Reference {
    name: SharedString,
    label: SharedString,
    group: &'static str,
    current: bool,
}

pub(super) struct BranchTree {
    query: Entity<Editor>,
    refs: Vec<Reference>,
    selected: Option<SharedString>,
    collapsed: HashSet<&'static str>,
    pub(super) hidden: bool,
    loading: bool,
    error: Option<String>,
    generation: u64,
    tag_scan_count: Option<usize>,
    task: Task<()>,
    scroll: UniformListScrollHandle,
    _query_observer: Subscription,
}

#[derive(Clone)]
enum Row {
    All,
    Group(&'static str, usize),
    Reference(Reference),
}

impl BranchTree {
    pub(super) fn new(window: &mut Window, cx: &mut Context<GitHistory>) -> Self {
        let query = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Branch or tag", window, cx);
            editor
        });
        let observer = cx.observe(&query, |_, _, cx| cx.notify());
        Self {
            query,
            refs: Vec::new(),
            selected: None,
            collapsed: HashSet::new(),
            hidden: false,
            loading: false,
            error: None,
            generation: 0,
            tag_scan_count: None,
            task: Task::ready(()),
            scroll: UniformListScrollHandle::new(),
            _query_observer: observer,
        }
    }

    pub(super) fn reset(&mut self) {
        self.generation += 1;
        self.task = Task::ready(());
        self.refs.clear();
        self.tag_scan_count = None;
        self.selected = None;
        self.error = None;
        self.loading = false;
    }

    pub(super) fn invalidate_tags(&mut self) {
        self.tag_scan_count = None;
        self.refs.retain(|entry| entry.group != "Tags");
    }

    pub(super) fn selected_tag(&self) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|name| name.starts_with("refs/tags/"))
    }

    fn rows(&self, cx: &App) -> Vec<Row> {
        let query = self.query.read(cx).text(cx).trim().to_lowercase();
        let mut rows = vec![Row::All];
        for group in ["Local", "Remote", "Tags"] {
            let refs = self
                .refs
                .iter()
                .filter(|entry| entry.group == group && entry.label.to_lowercase().contains(&query))
                .collect::<Vec<_>>();
            if refs.is_empty() {
                continue;
            }
            rows.push(Row::Group(group, refs.len()));
            if !self.collapsed.contains(group) || !query.is_empty() {
                rows.extend(refs.into_iter().cloned().map(Row::Reference));
            }
        }
        rows
    }
}

impl GitHistory {
    pub(super) fn reload_branches(&mut self, cx: &mut Context<Self>) {
        self.branches.generation += 1;
        self.branches.tag_scan_count = None;
        let generation = self.branches.generation;
        let Some(repository) = self.git.read(cx).active_repository() else {
            return;
        };
        let repository_id = repository.read(cx).id;
        let request = repository.update(cx, |repository, cx| {
            repository.graph_data(LogSource::All, LogOrder::default(), 0..0, cx);
            repository.branches()
        });
        self.branches.loading = true;
        self.branches.error = None;
        self.branches.task = cx.spawn(async move |history, cx| {
            let result = request.await.unwrap_or_else(|error| Err(error.into()));
            history
                .update(cx, |history, cx| {
                    if history.repository_id != Some(repository_id)
                        || history.branches.generation != generation
                    {
                        return;
                    }
                    history.branches.loading = false;
                    match result {
                        Ok(result) => {
                            history.branches.error = result.error.map(|error| error.to_string());
                            history.branches.refs.retain(|entry| entry.group == "Tags");
                            history
                                .branches
                                .refs
                                .extend(result.branches.into_iter().map(|branch| Reference {
                                    group: if branch.is_remote() {
                                        "Remote"
                                    } else {
                                        "Local"
                                    },
                                    label: branch.name().to_string().into(),
                                    name: branch.ref_name,
                                    current: branch.is_head,
                                }));
                            history.branches.refs.sort_by(|left, right| {
                                left.group
                                    .cmp(right.group)
                                    .then(left.label.cmp(&right.label))
                            });
                            history
                                .branches
                                .refs
                                .dedup_by(|left, right| left.name == right.name);
                            if history.branches.selected.as_ref().is_some_and(|selected| {
                                !selected.starts_with("refs/tags/")
                                    && !history
                                        .branches
                                        .refs
                                        .iter()
                                        .any(|entry| &entry.name == selected)
                            }) {
                                history.select_reference(None, cx);
                            }
                        }
                        Err(error) => history.branches.error = Some(error.to_string()),
                    }
                    cx.notify();
                })
                .log_err();
        });
        cx.notify();
    }

    pub(super) fn merge_tag_refs(&mut self, cx: &App) {
        let Some(repository) = self.git.read(cx).active_repository() else {
            return;
        };
        let repository = repository.read(cx);
        let Some(data) = repository.get_graph_data(LogSource::All, LogOrder::default()) else {
            return;
        };
        if self.branches.tag_scan_count == Some(data.commit_data.len()) {
            return;
        }
        self.branches.tag_scan_count = Some(data.commit_data.len());
        let mut tags = data
            .commit_data
            .iter()
            .flat_map(|commit| commit.tag_names())
            .map(|name| Reference {
                name: format!("refs/tags/{name}").into(),
                label: name.to_string().into(),
                group: "Tags",
                current: false,
            })
            .collect::<Vec<_>>();
        tags.sort_by(|left, right| left.name.cmp(&right.name));
        tags.dedup_by(|left, right| left.name == right.name);
        self.branches.refs.retain(|entry| entry.group != "Tags");
        self.branches.refs.extend(tags);
    }

    pub(super) fn clear_reference_filter(&mut self, cx: &mut Context<Self>) {
        self.select_reference(None, cx);
    }

    fn select_reference(&mut self, reference: Option<SharedString>, cx: &mut Context<Self>) {
        self.branches.selected = reference.clone();
        if let Some(graph) = &self.graph {
            graph.update(cx, |graph, cx| {
                graph.set_zedx_log_source(reference.map(LogSource::Branch).unwrap_or_default(), cx)
            });
        }
        cx.notify();
    }

    fn prototype_branch_notice(&self, message: &'static str, cx: &mut Context<Self>) {
        self.workspace
            .update(cx, |workspace, cx| {
                workspace.show_toast(
                    workspace::Toast::new(
                        workspace::notifications::NotificationId::unique::<BranchTree>(),
                        message,
                    )
                    .autohide(),
                    cx,
                )
            })
            .log_err();
    }

    pub(super) fn available_log_width(&self, window: &Window, cx: &App) -> Pixels {
        let mut width = window.viewport_size().width - px(86.0);
        if window.viewport_size().width >= px(560.0) {
            if let Some(workspace) = self.workspace.upgrade() {
                for position in [DockPosition::Left, DockPosition::Right] {
                    let dock = workspace.read(cx).dock_at_position(position).read(cx);
                    if dock.is_open() {
                        width -= dock
                            .stored_active_panel_size(window, cx)
                            .unwrap_or(px(270.0))
                            .min(px(420.0))
                            + px(6.0);
                    }
                }
            }
        }
        width.max(px(0.0))
    }

    pub(super) fn render_branches(&self, cx: &mut Context<Self>) -> AnyElement {
        let rows = self.branches.rows(cx);
        let empty = rows.len() == 1;
        h_flex().debug_selector(|| "zedx-git-branches".into()).w(px(220.0)).min_w(px(180.0)).h_full().flex_none()
            .border_r_1().border_color(cx.theme().colors().border)
            .child(v_flex().w(px(28.0)).h_full().flex_none().gap_1().items_center().py_1().border_r_1().border_color(cx.theme().colors().border)
                .child(ui::zedx::tool_button("zedx-git-hide-branches", ui::zedx::Icon::Collapse, "Hide branches", cx)
                    .on_click(cx.listener(|this, _, _, cx| { this.branches.hidden = true; cx.notify(); })))
                .child(ui::IconButton::new("zedx-git-new-branch", IconName::Plus).tooltip(Tooltip::text("New Branch…"))
                    .on_click(cx.listener(|this, _, _, cx| this.prototype_branch_notice("Prototype: creating a branch is not connected", cx))))
                .child(ui::IconButton::new("zedx-git-delete-branch", IconName::Trash).tooltip(Tooltip::text("Delete Branch"))
                    .on_click(cx.listener(|this, _, _, cx| this.prototype_branch_notice("Prototype: deleting a branch is not connected", cx))))
                .child(ui::zedx::tool_button("zedx-git-branch-refresh", ui::zedx::Icon::Refresh, "Refresh branches", cx)
                    .on_click(cx.listener(|this, _, _, cx| this.reload_branches(cx)))))
            .child(v_flex().flex_1().min_w_0().h_full()
                .child(h_flex().h(px(32.0)).flex_none().px_2().border_b_1().border_color(cx.theme().colors().border)
                    .child(self.branches.query.clone()))
                .when(self.branches.loading, |column| column.child(Label::new("Loading branches…").size(ui::zedx::caption_size()).color(Color::Muted)))
                .when_some(self.branches.error.clone(), |column, error| column.child(Label::new(error).size(ui::zedx::caption_size()).color(Color::Warning)))
                .when(empty && !self.branches.loading, |column| column.child(div().px_2().py_1().child(Label::new("No branches match").size(ui::zedx::caption_size()).color(Color::Muted))))
                .child(uniform_list("zedx-git-reference-list", rows.len(), cx.processor(move |this, range: Range<usize>, _, cx| {
                    range.filter_map(|index| rows.get(index).cloned()).map(|row| {
                        let (label, selected, depth, icon) = match &row {
                            Row::All => ("All branches".into(), this.branches.selected.is_none(), 0.0, ui::zedx::Icon::Git),
                            Row::Group(group, count) => (format!("{group} ({count})").into(), false, 0.0,
                                if this.branches.collapsed.contains(group) { ui::zedx::Icon::Chevron } else { ui::zedx::Icon::Down }),
                            Row::Reference(entry) => (entry.label.clone(), this.branches.selected.as_ref() == Some(&entry.name), 12.0, ui::zedx::Icon::Git),
                        };
                        let selector = match &row { Row::All => "zedx-git-ref-all".to_string(), Row::Group(group, _) => format!("zedx-git-group-{group}"), Row::Reference(entry) => format!("zedx-git-ref-{}", entry.name) };
                        div().debug_selector(move || selector).child(ui::ButtonLike::new(row_id(&row))
                            .height(px(25.0).into()).full_width().tab_index(0isize).toggle_state(selected).aria_label(label.clone()).tooltip(Tooltip::text(label.clone()))
                            .child(h_flex().w_full().min_w_0().gap_1().pl(px(4.0 + depth)).pr_1()
                                .child(ui::zedx::icon(icon, cx).size(px(13.0)))
                                .child(Label::new(label).size(ui::zedx::caption_size()).truncate())
                                .when(matches!(&row, Row::Reference(entry) if entry.current), |row| row.child(Label::new("current").size(ui::zedx::caption_size()).color(Color::Muted))))
                            .on_click(cx.listener(move |this, _, _, cx| match &row {
                                Row::All => this.select_reference(None, cx),
                                Row::Group(group, _) => { if !this.branches.collapsed.remove(group) { this.branches.collapsed.insert(group); } cx.notify(); },
                                Row::Reference(entry) => this.select_reference(Some(entry.name.clone()), cx),
                            }))).into_any_element()
                    }).collect::<Vec<_>>()
                })).track_scroll(&self.branches.scroll).flex_1().min_h_0()))
            .into_any_element()
    }
}

fn row_id(row: &Row) -> SharedString {
    match row {
        Row::All => "all".into(),
        Row::Group(group, _) => (*group).into(),
        Row::Reference(entry) => entry.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    async fn setup(
        cx: &mut TestAppContext,
    ) -> (
        Entity<Workspace>,
        Entity<GitHistory>,
        &mut VisualTestContext,
    ) {
        let (workspace, cx) = crate::zedx_panels::tests::setup(cx).await;
        let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
        let fs = project.read_with(cx, |project, _| project.fs().clone());
        let path = std::path::Path::new(util::path!("/project/.git"));
        fs.as_fake().pause_events();
        fs.as_fake()
            .insert_tree(util::path!("/project"), serde_json::json!({".git": {}}))
            .await;
        fs.as_fake().set_branch_name(path, Some("main"));
        fs.as_fake()
            .insert_branches(path, &["refs/heads/feature/ui", "origin/main"]);
        fs.as_fake().set_graph_commits(
            path,
            vec![std::sync::Arc::new(
                git::repository::InitialGraphCommitData {
                    sha: git::Oid::try_from("1111111111111111111111111111111111111111").unwrap(),
                    parents: Default::default(),
                    ref_names: vec!["HEAD -> main".into(), "tag: v1.0".into()],
                },
            )],
        );
        fs.as_fake().unpause_events_and_flush();
        project
            .update(cx, |project, cx| project.git_scans_complete(cx))
            .await;
        workspace.update_in(cx, |workspace, window, cx| {
            workspace.set_panel_size_state::<GitHistory>(
                workspace::dock::PanelSizeState {
                    size: Some(px(360.0)),
                    flex: None,
                },
                window,
                cx,
            )
        });
        cx.dispatch_action(ShowGitHistory);
        cx.run_until_parked();
        cx.refresh().unwrap();
        let history = workspace.read_with(cx, |workspace, cx| {
            workspace.panel::<GitHistory>(cx).unwrap()
        });
        (workspace, history, cx)
    }

    #[gpui::test]
    async fn git_history_groups_real_refs_and_filters_without_checkout(cx: &mut TestAppContext) {
        let (_, history, cx) = setup(cx).await;
        for group in [
            "zedx-git-group-Local",
            "zedx-git-group-Remote",
            "zedx-git-group-Tags",
        ] {
            assert!(cx.debug_bounds(group).is_some(), "{group}");
        }
        let graph = history.read_with(cx, |history, _| history.graph.clone().unwrap());
        cx.dispatch_action(git_ui::git_graph::FocusSearch);
        cx.simulate_input("main");
        cx.run_until_parked();
        let bounds = cx
            .debug_bounds("zedx-git-ref-refs/heads/feature/ui")
            .unwrap();
        cx.simulate_click(bounds.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert_eq!(
            history.read_with(cx, |history, _| history.graph.as_ref().unwrap().entity_id()),
            graph.entity_id()
        );
        assert_eq!(
            graph.read_with(cx, |graph, _| graph.log_source_for_test().clone()),
            LogSource::Branch("refs/heads/feature/ui".into())
        );
        assert_eq!(
            graph.read_with(cx, |graph, cx| graph.zedx_search_text_for_test(cx)),
            "main"
        );
        history.read_with(cx, |history, cx| {
            let repository = history.git.read(cx).active_repository().unwrap();
            assert_eq!(repository.read(cx).branch.as_ref().unwrap().name(), "main");
        });
        let bounds = cx.debug_bounds("zedx-git-ref-refs/tags/v1.0").unwrap();
        cx.simulate_click(bounds.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            graph.read_with(cx, |graph, _| graph.log_source_for_test().clone()),
            LogSource::Branch("refs/tags/v1.0".into())
        );
        cx.refresh().unwrap();
        let all = cx.debug_bounds("zedx-git-ref-all").unwrap();
        cx.simulate_click(all.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            graph.read_with(cx, |graph, _| graph.log_source_for_test().clone()),
            LogSource::All
        );
    }

    #[gpui::test]
    async fn git_branch_tree_filters_collapses_and_yields_to_compact_layout(
        cx: &mut TestAppContext,
    ) {
        let (_, history, cx) = setup(cx).await;
        let local = cx.debug_bounds("zedx-git-group-Local").unwrap();
        cx.simulate_click(local.center(), gpui::Modifiers::none());
        cx.refresh().unwrap();
        assert!(
            cx.debug_bounds("zedx-git-ref-refs/heads/feature/ui")
                .is_none()
        );
        history.update_in(cx, |history, window, cx| {
            history
                .branches
                .query
                .update(cx, |query, cx| query.set_text("feature", window, cx))
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(
            cx.debug_bounds("zedx-git-ref-refs/heads/feature/ui")
                .is_some(),
            "search reveals matches in a collapsed group"
        );
        assert!(cx.debug_bounds("zedx-git-group-Remote").is_none());
        history.update_in(cx, |history, window, cx| {
            history
                .branches
                .query
                .update(cx, |query, cx| query.set_text("no-such-ref", window, cx))
        });
        cx.run_until_parked();
        history.read_with(cx, |history, cx| {
            assert_eq!(history.branches.rows(cx).len(), 1)
        });
        cx.dispatch_action(git_ui::git_graph::ToggleZedxBranches);
        cx.refresh().unwrap();
        assert!(cx.debug_bounds("zedx-git-branches").is_none());
        cx.dispatch_action(git_ui::git_graph::ToggleZedxBranches);
        cx.simulate_resize(gpui::size(px(390.0), px(500.0)));
        cx.refresh().unwrap();
        assert!(cx.debug_bounds("zedx-git-branches").is_none());
        assert!(cx.debug_bounds("zedx-git-filters").is_some());
        cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
        cx.refresh().unwrap();
        assert!(cx.debug_bounds("zedx-git-branches").is_some());
    }

    #[gpui::test]
    async fn git_branch_refresh_includes_late_refs(cx: &mut TestAppContext) {
        let (workspace, history, cx) = setup(cx).await;
        let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
        let fs = project.read_with(cx, |project, _| project.fs().clone());
        fs.as_fake().insert_branches(
            std::path::Path::new(util::path!("/project/.git")),
            &["release"],
        );
        cx.dispatch_action(git_ui::git_graph::RefreshZedxBranches);
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(history.read_with(cx, |history, _| {
            history
                .branches
                .refs
                .iter()
                .any(|entry| entry.label == "release")
        }));
    }
    #[gpui::test]
    async fn deleted_branch_filter_returns_to_all(cx: &mut TestAppContext) {
        let (workspace, history, cx) = setup(cx).await;
        history.update(cx, |history, cx| {
            history.select_reference(Some("refs/heads/feature/ui".into()), cx)
        });
        let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
        let fs = project.read_with(cx, |project, _| project.fs().clone());
        fs.as_fake()
            .with_git_state(
                std::path::Path::new(util::path!("/project/.git")),
                true,
                |state| {
                    state.branches.remove("refs/heads/feature/ui");
                },
            )
            .unwrap();
        cx.dispatch_action(git_ui::git_graph::RefreshZedxBranches);
        cx.run_until_parked();
        history.read_with(cx, |history, cx| {
            assert!(history.branches.selected.is_none());
            assert_eq!(
                history
                    .graph
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .log_source_for_test(),
                &LogSource::All
            );
        });
    }

    #[gpui::test]
    async fn side_docks_leave_log_controls_unclipped(cx: &mut TestAppContext) {
        let (workspace, _, cx) = setup(cx).await;
        cx.simulate_resize(gpui::size(px(1000.0), px(700.0)));
        workspace.update_in(cx, |workspace, window, cx| {
            let right =
                cx.new(|cx| workspace::dock::test::TestPanel::new(DockPosition::Right, 99, cx));
            workspace.add_panel(right, window, cx);
            workspace.focus_panel::<crate::zedx_panels::Structure>(window, cx);
            workspace.toggle_dock(DockPosition::Right, window, cx);
        });
        workspace.update_in(cx, |workspace, window, cx| {
            workspace.focus_panel::<GitHistory>(window, cx);
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds("zedx-git-branches").is_none());
        let panel = cx.debug_bounds("zedx-git-history-panel").unwrap();
        let filters = cx.debug_bounds("zedx-git-filters").unwrap();
        let query = cx.debug_bounds("zedx-git-log-query").unwrap();
        assert!(filters.right() <= panel.right());
        assert!(query.right() <= filters.right());
        assert!(query.size.width > px(40.0));
    }
}

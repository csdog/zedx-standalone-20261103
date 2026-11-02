use super::*;
#[path = "zedx_git_history_branches.rs"]
mod branches;
use branches::BranchTree;
use git_ui::git_graph::GitGraph;
use project::git_store::RepositoryEvent;
use project::git_store::{GitStore, GitStoreEvent, RepositoryId};
use workspace::zedx_actions::ShowGitHistory;

pub(crate) struct GitHistory {
    workspace: WeakEntity<Workspace>,
    git: Entity<GitStore>,
    pub(super) graph: Option<Entity<GitGraph>>,
    repository_id: Option<RepositoryId>,
    active: bool,
    focus: FocusHandle,
    zoomed: bool,
    branches: BranchTree,
    _subscription: Subscription,
}

impl GitHistory {
    pub(super) fn new(workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let git = workspace.project().read(cx).git_store().clone();
        let subscription =
            cx.subscribe_in(&git, window, |this: &mut Self, _, event, window, cx| {
                if this.active {
                    if let GitStoreEvent::RepositoryUpdated(id, event, _) = event {
                        if Some(*id) == this.repository_id {
                            match event {
                                RepositoryEvent::HeadChanged
                                | RepositoryEvent::BranchListChanged => this.reload_branches(cx),
                                RepositoryEvent::TagsChanged => {
                                    this.branches.invalidate_tags();
                                    if this.branches.selected_tag() {
                                        this.clear_reference_filter(cx);
                                    }
                                    this.reload_branches(cx);
                                }
                                RepositoryEvent::GraphEvent(_, _) => cx.notify(),
                                _ => {}
                            }
                        }
                    }
                }
                if this.active
                    && matches!(
                        event,
                        GitStoreEvent::ActiveRepositoryChanged(_)
                            | GitStoreEvent::RepositoryAdded
                            | GitStoreEvent::RepositoryRemoved(_)
                    )
                {
                    this.refresh(window, cx);
                }
            });
        Self {
            workspace: workspace.weak_handle(),
            git,
            graph: None,
            repository_id: None,
            active: false,
            focus: cx.focus_handle(),
            zoomed: false,
            branches: BranchTree::new(window, cx),
            _subscription: subscription,
        }
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let repository_id = self
            .git
            .read(cx)
            .active_repository()
            .map(|repository| repository.read(cx).id);
        if self.repository_id == repository_id {
            return;
        }
        self.repository_id = repository_id;
        self.branches.reset();
        self.graph = repository_id.map(|repository_id| {
            cx.new(|cx| {
                let mut graph = GitGraph::new(
                    repository_id,
                    self.git.clone(),
                    self.workspace.clone(),
                    None,
                    window,
                    cx,
                );
                graph.use_zedx_toolbar(window, cx);
                graph
            })
        });
        self.reload_branches(cx);
        cx.notify();
    }

    fn activation_focus(&self, cx: &App) -> FocusHandle {
        self.graph
            .as_ref()
            .map(|graph| graph.focus_handle(cx))
            .unwrap_or_else(|| self.focus.clone())
    }

    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active {
            self.refresh(window, cx);
        }
    }

    fn badge(&self, _: &App) -> Option<String> {
        None
    }
}

fixed_panel!(
    GitHistory,
    "Git history",
    "ZedXGitHistory",
    DockPosition::Bottom,
    202.0,
    IconName::GitBranch,
    ShowGitHistory,
    6
);

impl Render for GitHistory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.merge_tag_refs(cx);
        let available_width = self.available_log_width(window, cx);
        let show_branches = !self.branches.hidden
            && window.viewport_size().width >= px(760.0)
            && available_width >= px(600.0);
        if let Some(graph) = &self.graph {
            graph.update(cx, |graph, cx| {
                graph.set_zedx_toolbar_width(
                    available_width - if show_branches { px(220.0) } else { px(0.0) },
                    cx,
                )
            });
        }
        v_flex()
            .on_action(
                cx.listener(|this, _: &git_ui::git_graph::FocusSearch, window, cx| {
                    if let Some(graph) = &this.graph {
                        window.focus(&graph.read(cx).focus_handle(cx), cx);
                    }
                }),
            )
            .on_action(
                cx.listener(|this, _: &git_ui::git_graph::ToggleZedxBranches, _, cx| {
                    this.branches.hidden = !this.branches.hidden;
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|this, _: &git_ui::git_graph::RefreshZedxBranches, _, cx| {
                    this.reload_branches(cx)
                }),
            )
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .debug_selector(|| "zedx-git-history-panel".into())
            .map(|view| match &self.graph {
                Some(graph) => view.child(
                    h_flex()
                        .size_full()
                        .min_h_0()
                        .when(show_branches, |row| row.child(self.render_branches(cx)))
                        .child(div().flex_1().min_w_0().h_full().child(graph.clone())),
                ),
                None => view.child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .debug_selector(|| "zedx-git-history-no-repository".into())
                        .child(Label::new("No Git repository in this project").color(Color::Muted))
                        .child(
                            Label::new("Open a Git project to browse commits and changes.")
                                .size(ui::zedx::label_size())
                                .color(Color::Muted),
                        ),
                ),
            })
    }
}

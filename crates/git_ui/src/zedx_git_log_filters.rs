use super::*;
use anyhow::Context as _;
use git::repository::ZedxLogFilter;
use gpui::{DismissEvent, EventEmitter, Focusable};

#[derive(Default)]
pub(super) struct Metadata {
    pub(super) spec: ZedxLogFilter,
    pub(super) matches: collections::HashSet<Oid>,
    pub(super) loading: bool,
    pub(super) error: Option<String>,
    generation: u64,
    task: Option<Task<()>>,
}
pub(super) fn bind_keys(cx: &mut App) {
    for context in ["ZedxGitLogFilters", "ZedxGitLogFilters > Editor"] {
        cx.bind_keys([gpui::KeyBinding::new("escape", menu::Cancel, Some(context))]);
    }
}

impl GitGraph {
    pub(super) fn open_zedx_log_filters(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let graph = cx.weak_entity();
        let spec = self.zedx_filter.metadata.spec.clone();
        self.workspace
            .update(cx, |workspace, cx| {
                workspace.toggle_modal(window, cx, move |window, cx| {
                    FilterDialog::new(graph, spec, index, window, cx)
                });
            })
            .log_err();
    }
    pub fn refresh_zedx_log_filters(&mut self, cx: &mut Context<Self>) {
        let spec = self.zedx_filter.metadata.spec.clone();
        self.zedx_filter.metadata.generation += 1;
        let generation = self.zedx_filter.metadata.generation;
        self.zedx_filter.metadata.task = None;
        self.zedx_filter.metadata.matches.clear();
        self.zedx_filter.metadata.loading = !spec.is_empty();
        self.zedx_filter.metadata.error = None;
        self.zedx_filter.projection = None;
        self.clear_zedx_commit_selection();
        if spec.is_empty() {
            cx.notify();
            return;
        }
        let Some(repository) = self.get_repository(cx) else {
            return;
        };
        let source = self.log_source.clone();
        let request = repository.update(cx, |repository, _| {
            repository.zedx_filter_commits(source.clone(), spec)
        });
        self.zedx_filter.metadata.task = Some(cx.spawn(async move |graph, cx| {
            let result = request.await.unwrap_or_else(|error| Err(error.into()));
            graph
                .update(cx, |graph, cx| {
                    if graph.zedx_filter.metadata.generation != generation
                        || graph.log_source != source
                    {
                        return;
                    }
                    graph.zedx_filter.metadata.loading = false;
                    match result {
                        Ok(matches) => {
                            graph.zedx_filter.metadata.matches.extend(matches);
                            if let Some(index) = graph.zedx_filtered_indices().first().copied() {
                                graph.select_entry(index, ScrollStrategy::Top, cx);
                            }
                        }
                        Err(error) => graph.zedx_filter.metadata.error = Some(error.to_string()),
                    }
                    cx.notify();
                })
                .log_err();
        }));
        cx.notify();
    }
}

struct FilterDialog {
    graph: WeakEntity<GitGraph>,
    editors: [Entity<Editor>; 4],
    focus: FocusHandle,
    error: Option<String>,
}
impl FilterDialog {
    fn new(
        graph: WeakEntity<GitGraph>,
        spec: ZedxLogFilter,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let values = [spec.author, spec.after, spec.before, spec.paths.join("\n")];
        let editors = std::array::from_fn(|index| {
            cx.new(|cx| {
                let mut editor = if index == 3 {
                    Editor::multi_line(window, cx)
                } else {
                    Editor::single_line(window, cx)
                };
                editor.set_text(values[index].clone(), window, cx);
                editor
            })
        });
        let focus = editors[index.min(3)].focus_handle(cx);
        Self {
            graph,
            editors,
            focus,
            error: None,
        }
    }
    fn apply(&mut self, clear: bool, cx: &mut Context<Self>) {
        let spec = if clear {
            ZedxLogFilter::default()
        } else {
            ZedxLogFilter {
                author: self.editors[0].read(cx).text(cx).trim().into(),
                after: self.editors[1].read(cx).text(cx).trim().into(),
                before: self.editors[2].read(cx).text(cx).trim().into(),
                paths: self.editors[3]
                    .read(cx)
                    .text(cx)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToString::to_string)
                    .collect(),
            }
        };
        if let Err(error) = spec.validate() {
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        let result = self
            .graph
            .update(cx, |graph, cx| {
                graph.zedx_filter.metadata.spec = spec;
                graph.refresh_zedx_log_filters(cx);
            })
            .context("The Git log was closed");
        match result {
            Ok(()) => cx.emit(DismissEvent),
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }
}
impl EventEmitter<DismissEvent> for FilterDialog {}
impl Focusable for FilterDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl ModalView for FilterDialog {}
impl Render for FilterDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(420.0))
            .max_w_full()
            .p_3()
            .gap_2()
            .track_focus(&self.focus)
            .key_context("ZedxGitLogFilters")
            .on_action(cx.listener(|_, _: &menu::Cancel, _, cx| cx.emit(DismissEvent)))
            .child(Label::new("Filter Git Log").size(ui::zedx::label_size()))
            .children(
                [
                    "User (author name or email)",
                    "From date (YYYY-MM-DD)",
                    "Through date (YYYY-MM-DD)",
                    "Paths (one repository-relative path per line)",
                ]
                .into_iter()
                .enumerate()
                .map(|(index, label)| {
                    v_flex()
                        .gap_1()
                        .child(Label::new(label).size(ui::zedx::caption_size()))
                        .child(
                            div()
                                .h(px(if index == 3 { 84.0 } else { 28.0 }))
                                .p_1()
                                .border_1()
                                .rounded_sm()
                                .border_color(cx.theme().colors().border)
                                .child(self.editors[index].clone()),
                        )
                }),
            )
            .when_some(self.error.clone(), |view, error| {
                view.child(Label::new(error).color(Color::Error))
            })
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("apply-git-filters", "Apply")
                            .on_click(cx.listener(|this, _, _, cx| this.apply(false, cx))),
                    )
                    .child(
                        Button::new("clear-git-filters", "Clear")
                            .on_click(cx.listener(|this, _, _, cx| this.apply(true, cx))),
                    )
                    .child(
                        Button::new("cancel-git-filters", "Cancel")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(DismissEvent))),
                    ),
            )
    }
}

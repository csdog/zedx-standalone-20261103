use super::*;
use ui::HighlightedLabel;

impl SearchWindow {
    fn render_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let delegate = &self.picker.read(cx).delegate;
        let include_non_project = delegate.scope_includes_extra();
        let file_filter = delegate.file_filter;
        let actions = delegate.category == Category::Actions;
        let scope_label = if actions {
            "Include disabled actions"
        } else {
            "Include non-project items"
        };
        let weak = cx.entity().downgrade();
        h_flex()
            .w_full()
            .flex_wrap()
            .px_3()
            .py_1()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .when(!matches!(delegate.category, Category::All | Category::Actions), |controls| controls.child(div().debug_selector(|| "zedx-search-scope".into()).child(Button::new("search-scope", if include_non_project { "All Places" } else { "Project" })
                .label_size(ui::zedx::label_size())
                .tooltip(Tooltip::text("Search project files, or include ignored files and language-server dependencies"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.picker.update(cx, |picker, _| {
                        let include = !picker.delegate.scope_includes_extra();
                        picker.delegate.set_scope_includes_extra(include);
                    });
                    this.refresh(window, cx);
                })))))
            .when(matches!(delegate.category, Category::All | Category::Actions), |controls| controls.child(div().debug_selector(|| "zedx-search-include".into()).child(Checkbox::new("search-non-project", if include_non_project { ToggleState::Selected } else { ToggleState::Unselected })
                .label(scope_label)
                .label_size(ui::zedx::caption_size())
                .on_click(cx.listener(|this, state, window, cx| {
                    this.picker.update(cx, |picker, _| picker.delegate.set_scope_includes_extra(*state == ToggleState::Selected));
                    this.refresh(window, cx);
                })))))
            .child(div().flex_1())
            .when(!matches!(delegate.category, Category::All | Category::Actions), |controls| controls.child(div().debug_selector(|| "zedx-search-file-filter".into()).child(PopoverMenu::new("search-file-filter")
                .menu_priority(3)
                .trigger_with_tooltip(
                    IconButton::new("search-file-filter-button", IconName::Filter)
                        .disabled(actions)
                        .toggle_state(!file_filter.is_all()),
                    Tooltip::text(file_filter.label()),
                )
                .menu(move |window, cx| {
                    let weak = weak.clone();
                    Some(ContextMenu::build_persistent(window, cx, move |mut menu, _, cx| {
                        let current = weak.read_with(cx, |this, cx| this.picker.read(cx).delegate.file_filter).unwrap_or_default();
                        for filter in FileFilter::ALL {
                            let weak = weak.clone();
                            menu = menu.toggleable_entry(filter.label(), current.contains(filter), IconPosition::Start, None, move |window, cx| {
                                weak.update(cx, |this, cx| {
                                    this.picker.update(cx, |picker, cx| {
                                        picker.delegate.file_filter.toggle(filter);
                                        picker.update_matches(picker.query(cx), window, cx);
                                    });
                                    cx.notify();
                                }).log_err();
                            });
                        }
                        menu
                    }))
                }))))
            .child(IconButton::new("search-clear", IconName::Close)
                .aria_label("Clear search")
                .tooltip(Tooltip::text("Clear search"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.picker.update(cx, |picker, cx| picker.set_query("", window, cx));
                    window.focus(&this.picker.focus_handle(cx), cx);
                })))
            .child(IconButton::new("search-find-in-files", IconName::MagnifyingGlass)
                .aria_label("Find in Files")
                .disabled(actions)
                .tooltip(Tooltip::text("Find in Files"))
                .on_click(cx.listener(|this, _, window, cx| this.find_in_files(window, cx))))
            .child(IconButton::new("search-split", IconName::Split)
                .aria_label("Open in right split")
                .disabled(actions || self.picker.read(cx).delegate.rows.is_empty())
                .tooltip(Tooltip::text("Open in right split · Shift+Enter"))
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.picker.focus_handle(cx), cx);
                    window.dispatch_action(menu::SecondaryConfirm.boxed_clone(), cx);
                })))
    }
}

impl Render for SearchWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let compact = viewport.width < px(760.0);
        let height_fraction = if compact { 0.85 } else { 0.82 };
        let width =
            px(800.0).min((viewport.width - px(if compact { 24.0 } else { 36.0 })).max(px(1.0)));
        let top = viewport.height * if compact { 0.07 } else { 0.09 };
        let header_height = if width < px(570.0) {
            px(100.0)
        } else {
            px(75.0)
        };
        let available_height = (viewport.height * height_fraction - header_height).max(px(1.0));
        let delegate = &self.picker.read(cx).delegate;
        let groups = (0..delegate.rows.len())
            .filter(|index| *index == 0 || delegate.section(*index - 1) != delegate.section(*index))
            .count();
        let results_height = px(
            (delegate.rows.len() as f32 * if compact { 27.0 } else { 26.0 } + groups as f32 * 25.0)
                .clamp(70.0, if compact { 280.0 } else { 336.0 })
                + 75.0,
        );
        self.picker.update(cx, |picker, _| {
            picker.fit_zedx_search(width, available_height, results_height, window)
        });
        let origin = self.placement.position(viewport, width, top);
        let modal = v_flex()
            .id("zedx-search-modal")
            .debug_selector(|| "zedx-search-modal".into())
            .w(width)
            .max_h(viewport.height * height_fraction)
            .absolute()
            .left(origin.x)
            .top(origin.y)
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .key_context("ZedXSearch")
            .max_w_full()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().elevated_surface_background)
            .shadow_lg()
            .overflow_hidden()
            .on_action(
                cx.listener(|this, _: &NextSearchCategory, window, cx| {
                    this.cycle(false, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &PreviousSearchCategory, window, cx| {
                this.cycle(true, window, cx)
            }))
            .child(
                h_flex()
                    .w_full()
                    .px_3()
                    .pt_2()
                    .gap_1()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        h_flex()
                            .id("search-categories")
                            .w(px(390.0).min((width - px(90.0)).max(px(1.0))))
                            .min_w_0()
                            .overflow_x_scroll()
                            .children(Category::ALL.into_iter().enumerate().map(
                                |(index, category)| {
                                    Button::new(("search-category", index), category.label())
                                        .label_size(ui::zedx::label_size())
                                        .toggle_state(
                                            category == self.picker.read(cx).delegate.category,
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.picker.update(cx, |picker, cx| {
                                                picker.delegate.category = category;
                                                picker.update_matches(picker.query(cx), window, cx);
                                            });
                                            window.focus(&this.picker.focus_handle(cx), cx);
                                            cx.notify();
                                        }))
                                },
                            )),
                    )
                    .child(
                        div()
                            .id("zedx-search-drag-handle")
                            .debug_selector(|| "zedx-search-drag-handle".into())
                            .flex_1()
                            .min_w(px(24.0))
                            .h(px(24.0))
                            .cursor_move()
                            .aria_label("Drag search window")
                            .tooltip(Tooltip::text("Drag search window"))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                                    this.placement.begin(event.position);
                                    cx.stop_propagation();
                                }),
                            ),
                    )
                    .child(
                        IconButton::new("search-preview", IconName::Eye)
                            .aria_label("Toggle code preview")
                            .tooltip(Tooltip::text("Toggle code preview"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.picker.focus_handle(cx), cx);
                                window.dispatch_action(picker::TogglePreview.boxed_clone(), cx);
                            })),
                    ),
            )
            .child(self.render_controls(cx))
            .child(self.picker.clone());
        let bounds = self.placement.bounds.clone();
        let weak = cx.entity().downgrade();
        let layer = h_flex()
            .on_children_prepainted(move |children, _, cx| {
                if let Some(bound) = children.last().copied() {
                    let changed = bounds.get().is_none_or(|old| old.size != bound.size);
                    bounds.set(Some(bound));
                    if changed {
                        let weak = weak.clone();
                        cx.defer(move |cx| {
                            weak.update(cx, |this, cx| {
                                if this.placement.fit(viewport) {
                                    cx.notify();
                                }
                            })
                            .log_err();
                        });
                    }
                }
            })
            .absolute()
            .inset_0()
            .size_full()
            .items_start()
            .justify_center()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.emit(DismissEvent)),
            )
            .child(self.drag_tracking(cx))
            .child(modal);
        gpui::deferred(layer).with_priority(2)
    }
}

impl Row {
    pub(super) fn group(&self) -> (u8, &'static str) {
        match &self.target {
            Target::Symbol(symbol)
                if matches!(
                    symbol.kind,
                    SymbolKind::Class
                        | SymbolKind::Struct
                        | SymbolKind::Interface
                        | SymbolKind::Enum
                ) =>
            {
                (0, "Classes")
            }
            Target::File(_) => (1, "Files"),
            Target::Symbol(_) => (2, "Symbols"),
            Target::Action(_) => (3, "Actions"),
            Target::Text(..) => (4, "Text"),
        }
    }
    pub(super) fn same_target(&self, other: &Self) -> bool {
        match (&self.target, &other.target) {
            (Target::File(left), Target::File(right)) => left == right,
            (Target::Action(left), Target::Action(right)) => left == right,
            (Target::Text(left_buffer, left_range), Target::Text(right_buffer, right_range)) => {
                left_buffer == right_buffer && left_range == right_range
            }
            (Target::Symbol(left), Target::Symbol(right)) => {
                left.range == right.range
                    && self.label == other.label
                    && self.detail == other.detail
            }
            _ => false,
        }
    }
}

impl SearchDelegate {
    pub(super) fn section(&self, index: usize) -> &'static str {
        if self.query.is_empty()
            && matches!(self.category, Category::All | Category::Files)
            && self
                .rows
                .get(index)
                .is_some_and(|row| matches!(row.target, Target::File(_)))
        {
            return "Recent Files";
        }
        if self.category != Category::All {
            return self.category.label();
        }
        self.rows
            .get(index)
            .map(|row| row.group().1)
            .unwrap_or_default()
    }
    pub(super) fn render_row(
        &self,
        index: usize,
        selected: bool,
        window: &Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<AnyElement> {
        let row = self.rows.get(index)?;
        let disabled = self.action_is_disabled(&row.target);
        let section = self.section(index);
        let starts_group = index == 0 || self.section(index - 1) != section;
        Some(
            v_flex()
                .debug_selector(|| format!("zedx-search-row-{index}"))
                .w_full()
                .min_w_0()
                .when(starts_group, |column| {
                    column.child(
                        h_flex()
                            .id(("search-section", index))
                            .h(px(25.0))
                            .px_3()
                            .gap_2()
                            .debug_selector(|| {
                                format!(
                                    "zedx-search-group-{}",
                                    section.to_lowercase().replace(' ', "-")
                                )
                            })
                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation()
                            })
                            .on_mouse_up(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Label::new(section)
                                    .size(ui::zedx::caption_size())
                                    .color(Color::Muted),
                            )
                            .child(div().h_px().flex_1().bg(cx.theme().colors().border_variant)),
                    )
                })
                .child(
                    ListItem::new(index)
                        .disabled(disabled)
                        .spacing(ListItemSpacing::Dense)
                        .height(px(if window.viewport_size().width < px(760.0) {
                            27.0
                        } else {
                            26.0
                        }))
                        .toggle_state(selected)
                        .child(
                            h_flex()
                                .when(disabled, |row| {
                                    row.opacity(0.5)
                                        .debug_selector(|| "zedx-search-disabled-action".into())
                                })
                                .w_full()
                                .min_w_0()
                                .gap_2()
                                .child(match &row.icon {
                                    Some(icon) => ui::zedx::asset(icon.clone())
                                        .size(px(16.0))
                                        .into_any_element(),
                                    None => Icon::new(match row.target {
                                        Target::Action(_) => IconName::BoltOutlined,
                                        _ => IconName::Code,
                                    })
                                    .size(IconSize::Small)
                                    .color(Color::Muted)
                                    .into_any_element(),
                                })
                                .child(
                                    div()
                                        .flex_shrink(1.0)
                                        .min_w_0()
                                        .max_w(relative(0.68))
                                        .child(
                                            HighlightedLabel::new(
                                                row.label.clone(),
                                                highlight_indices(&row.label, &self.query),
                                            )
                                            .size(ui::zedx::label_size())
                                            .truncate(),
                                        ),
                                )
                                .child(
                                    div().flex_1().min_w_0().child(
                                        HighlightedLabel::new(
                                            row.detail.clone(),
                                            highlight_indices(&row.detail, &self.query),
                                        )
                                        .size(ui::zedx::caption_size())
                                        .color(Color::Muted)
                                        .truncate(),
                                    ),
                                )
                                .child(
                                    div().flex_none().max_w(relative(0.25)).child(
                                        Label::new(row.source.clone())
                                            .size(ui::zedx::caption_size())
                                            .color(Color::Muted)
                                            .truncate(),
                                    ),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}

impl SearchDelegate {
    pub(super) fn render_source_footer(
        &self,
        window: &Window,
        cx: &mut Context<Picker<Self>>,
    ) -> AnyElement {
        let compact = window.viewport_size().width < px(700.0);
        let disabled = self
            .rows
            .get(self.selected)
            .is_some_and(|row| self.action_is_disabled(&row.target));
        h_flex()
            .debug_selector(|| "zedx-search-footer".into())
            .w_full()
            .min_w_0()
            .gap_3()
            .px_3()
            .h(px(30.0))
            .border_t_1()
            .border_color(cx.theme().colors().border)
            .child(
                div()
                    .id("zedx-search-status")
                    .debug_selector(|| "zedx-search-status".into())
                    .min_w_0()
                    .max_w(px(if compact { 140.0 } else { 360.0 }))
                    .flex_shrink(1.0)
                    .when_some(self.error.clone(), |status, error| {
                        status.tooltip(Tooltip::text(error))
                    })
                    .child(
                        Label::new(self.error.clone().unwrap_or_else(|| {
                            format!(
                                "{} {}",
                                self.rows.len(),
                                if self.rows.len() == 1 {
                                    "result"
                                } else {
                                    "results"
                                }
                            )
                        }))
                        .size(ui::zedx::caption_size())
                        .truncate()
                        .color(Color::Muted),
                    ),
            )
            .child(
                div().flex_1().min_w_0().child(
                    Label::new(
                        self.rows
                            .get(self.selected)
                            .map(|row| row.detail.clone())
                            .unwrap_or_default(),
                    )
                    .size(ui::zedx::caption_size())
                    .color(Color::Muted)
                    .truncate(),
                ),
            )
            .when(
                !compact
                    && self
                        .rows
                        .get(self.selected)
                        .is_some_and(|row| !matches!(row.target, Target::Action(_))),
                |bar| {
                    bar.child(
                        Button::new("search-open-right", "Open in Right Split")
                            .label_size(ui::zedx::caption_size())
                            .on_click(cx.listener(|picker, _, window, cx| {
                                window.focus(&picker.focus_handle(cx), cx);
                                window.dispatch_action(menu::SecondaryConfirm.boxed_clone(), cx);
                            })),
                    )
                },
            )
            .child(
                Label::new(if disabled {
                    "Unavailable · esc Close"
                } else if compact {
                    "↵ Open · esc Close"
                } else {
                    "↑↓ Navigate    ↵ Open    ⇧↵ Split    ⇥ Category    esc Close"
                })
                .size(ui::zedx::caption_size())
                .color(Color::Muted),
            )
            .into_any_element()
    }
}

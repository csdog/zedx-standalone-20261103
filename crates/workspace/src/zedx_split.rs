#[path = "zedx_split_persistence.rs"]
mod persistence;

use super::*;
use gpui::{Bounds, DragMoveEvent, Point, canvas, relative};
use std::{cell::Cell, rc::Rc};

/// Tool-window payload shared by the rail and a window's draggable header.
#[derive(Clone)]
pub struct PanelDrag {
    pub name: String,
}

#[derive(Default)]
pub(super) struct SplitState {
    enabled: bool,
    resizing: bool,
    fraction: Option<f32>,
    persistence: persistence::Persistence,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

#[derive(Clone)]
struct ResizeSplit;
impl Render for ResizeSplit {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl Dock {
    fn zedx_split_panels(&self) -> Option<(&Arc<dyn PanelHandle>, &Arc<dyn PanelHandle>)> {
        if !crate::zedx_workbench::enabled() || self.position != DockPosition::Left {
            return None;
        }
        let project = self
            .panel_entries
            .iter()
            .find(|entry| entry.panel.persistent_name() == "Project Panel")?;
        let structure = self
            .panel_entries
            .iter()
            .find(|entry| entry.panel.persistent_name() == "Structure")?;
        Some((&project.panel, &structure.panel))
    }

    fn zedx_split_visible(&self) -> bool {
        self.zedx_split.enabled
            && self.is_open
            && self.zedx_split_panels().is_some()
            && self.active_panel().is_some_and(|panel| {
                matches!(panel.persistent_name(), "Project Panel" | "Structure")
            })
    }

    pub(crate) fn zedx_panel_visible(&self, panel: &dyn PanelHandle) -> bool {
        self.is_open
            && (self
                .active_panel()
                .is_some_and(|active| active.panel_id() == panel.panel_id())
                || (self.zedx_split_visible()
                    && matches!(panel.persistent_name(), "Project Panel" | "Structure")))
    }

    pub(super) fn sync_zedx_split(&self, window: &mut Window, cx: &mut App) {
        if self.zedx_split.enabled
            && let Some((project, structure)) = self.zedx_split_panels()
        {
            let visible = self.zedx_split_visible();
            for panel in [project, structure] {
                panel.set_active(visible, window, cx);
            }
        }
    }

    pub(crate) fn close_zedx_split_panel(
        &mut self,
        id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.zedx_split_visible() {
            return false;
        }
        let Some((project, structure)) = self.zedx_split_panels() else {
            return false;
        };
        let survivor = if id == structure.panel_id() {
            project.clone()
        } else if id == project.panel_id() {
            structure.clone()
        } else {
            return false;
        };
        for panel in [project, structure] {
            panel.set_active(false, window, cx);
        }
        self.zedx_split.enabled = false;
        self.persist_zedx_split(cx);
        if let Some(index) = self
            .panel_entries
            .iter()
            .position(|entry| entry.panel.panel_id() == survivor.panel_id())
        {
            self.activate_panel(index, window, cx);
        }
        survivor.set_active(true, window, cx);
        survivor.activation_focus_handle(cx).focus(window, cx);
        cx.notify();
        true
    }

    pub(crate) fn reset_zedx_split(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((project, structure)) = self.zedx_split_panels() {
            for panel in [project, structure] {
                panel.set_active(
                    self.is_open
                        && self
                            .active_panel()
                            .is_some_and(|active| active.panel_id() == panel.panel_id()),
                    window,
                    cx,
                );
            }
        }
        let persistence = std::mem::take(&mut self.zedx_split.persistence);
        self.zedx_split = SplitState {
            persistence,
            ..Default::default()
        };
        self.persist_zedx_split(cx);
        cx.notify();
    }

    pub(crate) fn toggle_zedx_split_pin(
        &mut self,
        id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.close_zedx_split_panel(id, window, cx)
    }

    fn split_structure_below_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((project, structure)) = self.zedx_split_panels() else {
            return;
        };
        if !project.enabled(cx) || !structure.enabled(cx) {
            return;
        }
        let structure = structure.clone();
        let Some(index) = self
            .panel_entries
            .iter()
            .position(|entry| entry.panel.panel_id() == structure.panel_id())
        else {
            return;
        };
        self.zedx_split.enabled = true;
        self.persist_zedx_split(cx);
        self.activate_panel(index, window, cx);
        self.set_open(true, window, cx);
        self.sync_zedx_split(window, cx);
        structure.activation_focus_handle(cx).focus(window, cx);
        cx.notify();
    }

    fn resize_zedx_split(&mut self, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(bounds) = self.zedx_split.bounds.get() {
            let height = f32::from(bounds.size.height).max(1.0);
            let minimum = (100.0 / height).min(0.4);
            self.zedx_split.fraction =
                Some((f32::from(pointer.y - bounds.top()) / height).clamp(minimum, 1.0 - minimum));
            cx.notify();
        }
    }

    pub(super) fn render_zedx_split_or_panel(
        &self,
        panel: &Arc<dyn PanelHandle>,
        window: &Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        if self.zedx_split_panels().is_none() {
            return self.render_zedx_panel(panel, cx);
        }
        let bounds = self.zedx_split.bounds.clone();
        let weak = cx.weak_entity();
        let content = if let Some((project, structure)) = self
            .zedx_split_panels()
            .filter(|_| self.zedx_split_visible())
        {
            let height = bounds
                .get()
                .map_or(window.viewport_size().height, |bounds| bounds.size.height);
            let available = (height - px(5.0)).max(px(0.0));
            let minimum = (100.0 / f32::from(available).max(1.0)).min(0.4);
            let fraction = self
                .zedx_split
                .fraction
                .unwrap_or(0.6)
                .clamp(minimum, 1.0 - minimum);
            ui::v_flex()
                .size_full()
                .min_h_0()
                .child(
                    div()
                        .debug_selector(|| "zedx-split-project".into())
                        .w_full()
                        .h(available * fraction)
                        .flex_none()
                        .overflow_hidden()
                        .child(self.render_zedx_panel(project, cx)),
                )
                .child(
                    div()
                        .id("zedx-left-split-resize")
                        .debug_selector(|| "zedx-left-split-resize".into())
                        .h(px(5.0))
                        .w_full()
                        .flex_none()
                        .cursor_row_resize()
                        .bg(cx.theme().colors().border_variant)
                        .hover(|style| style.bg(cx.theme().colors().element_hover))
                        .on_drag(ResizeSplit, |drag, _, _, cx| cx.new(|_| drag.clone()))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseUpEvent, _, cx| {
                                if event.click_count == 2 {
                                    this.zedx_split.fraction = None;
                                    this.persist_zedx_split(cx);
                                    cx.notify();
                                }
                            }),
                        ),
                )
                .child(
                    div()
                        .debug_selector(|| "zedx-split-structure".into())
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .overflow_hidden()
                        .child(self.render_zedx_panel(structure, cx)),
                )
                .into_any_element()
        } else {
            self.render_zedx_panel(panel, cx)
        };
        div()
            .id("zedx-left-split-host")
            .debug_selector(|| "zedx-left-split-host".into())
            .relative()
            .size_full()
            .on_drag_move::<ResizeSplit>(cx.listener(
                |this, event: &DragMoveEvent<ResizeSplit>, _, cx| {
                    this.zedx_split.resizing = true;
                    this.resize_zedx_split(event.event.position, cx)
                },
            ))
            .on_drop::<ResizeSplit>(cx.listener(|this, _, window, cx| {
                this.resize_zedx_split(window.mouse_position(), cx);
                this.zedx_split.resizing = false;
                this.persist_zedx_split(cx);
            }))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.zedx_split.resizing {
                        this.zedx_split.resizing = false;
                        this.persist_zedx_split(cx);
                    }
                }),
            )
            .child(
                canvas(
                    move |current, window, cx| {
                        if bounds.replace(Some(current)) != Some(current) {
                            window.defer(cx, move |_, cx| {
                                weak.update(cx, |_, cx| cx.notify()).log_err();
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .child(content)
            .on_drop::<PanelDrag>(cx.listener(|this, drag: &PanelDrag, window, cx| {
                if drag.name == "Structure"
                    && let Some(bounds) = this.zedx_split.bounds.get()
                    && bounds.contains(&window.mouse_position())
                    && window.mouse_position().y >= bounds.center().y
                {
                    this.split_structure_below_project(window, cx);
                }
            }))
            .when(cx.has_active_drag(), |host| {
                host.child(
                    div()
                        .id("zedx-project-lower-drop")
                        .debug_selector(|| "zedx-project-lower-drop".into())
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(relative(0.5))
                        .bottom_0()
                        .can_drop(|drag, _, _| {
                            drag.downcast_ref::<PanelDrag>()
                                .is_some_and(|drag| drag.name == "Structure")
                        })
                        .drag_over::<PanelDrag>(|style, drag, _, cx| {
                            if drag.name == "Structure" {
                                style
                                    .bg(cx.theme().colors().element_selected.opacity(0.65))
                                    .border_2()
                                    .border_color(cx.theme().colors().border_focused)
                            } else {
                                style
                            }
                        })
                        .on_drop::<PanelDrag>(cx.listener(|this, drag: &PanelDrag, window, cx| {
                            if drag.name == "Structure" {
                                this.split_structure_below_project(window, cx);
                            }
                        })),
                )
            })
            .into_any_element()
    }
}

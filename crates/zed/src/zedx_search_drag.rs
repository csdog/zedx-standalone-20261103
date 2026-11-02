use gpui::{Bounds, Pixels, Point, Size, point, px};
use std::{cell::Cell, rc::Rc};

#[derive(Default)]
pub(super) struct Placement {
    pub(super) bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    origin: Option<Point<Pixels>>,
    drag: Option<(Point<Pixels>, Point<Pixels>)>,
}

impl Placement {
    pub(super) fn position(
        &mut self,
        viewport: Size<Pixels>,
        width: Pixels,
        top: Pixels,
    ) -> Point<Pixels> {
        let Some(origin) = self.origin else {
            return point((viewport.width - width) / 2.0, top);
        };
        let height = self
            .bounds
            .get()
            .map_or(px(1.0), |bounds| bounds.size.height);
        let origin = clamp(origin, gpui::size(width, height), viewport);
        self.origin = Some(origin);
        origin
    }

    pub(super) fn begin(&mut self, pointer: Point<Pixels>) {
        if let Some(bounds) = self.bounds.get() {
            self.origin = Some(bounds.origin);
            self.drag = Some((pointer, bounds.origin));
        }
    }

    pub(super) fn update(&mut self, pointer: Point<Pixels>, viewport: Size<Pixels>) -> bool {
        let Some((start, origin)) = self.drag else {
            return false;
        };
        let Some(bounds) = self.bounds.get() else {
            return false;
        };
        let origin = clamp(origin + pointer - start, bounds.size, viewport);
        let changed = self.origin != Some(origin);
        self.origin = Some(origin);
        changed
    }

    pub(super) fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub(super) fn finish(&mut self) {
        self.drag = None;
    }

    pub(super) fn fit(&mut self, viewport: Size<Pixels>) -> bool {
        let (Some(origin), Some(bounds)) = (self.origin, self.bounds.get()) else {
            return false;
        };
        let fitted = clamp(origin, bounds.size, viewport);
        self.origin = Some(fitted);
        fitted != origin
    }
}

fn clamp(origin: Point<Pixels>, size: Size<Pixels>, viewport: Size<Pixels>) -> Point<Pixels> {
    let margin = px(8.0);
    point(
        origin
            .x
            .clamp(margin, (viewport.width - size.width - margin).max(margin)),
        origin
            .y
            .clamp(margin, (viewport.height - size.height - margin).max(margin)),
    )
}

impl super::SearchWindow {
    pub(super) fn drag_tracking(&self, cx: &gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::Styled;
        use util::ResultExt;
        let weak = cx.entity().downgrade();
        gpui::canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                window.on_mouse_event({
                    let weak = weak.clone();
                    move |event: &gpui::MouseMoveEvent, phase, window, cx| {
                        if !phase.capture() {
                            return;
                        }
                        weak.update(cx, |this, cx| {
                            if !this.placement.is_dragging() {
                                return;
                            }
                            if event.pressed_button == Some(gpui::MouseButton::Left) {
                                if this
                                    .placement
                                    .update(event.position, window.viewport_size())
                                {
                                    cx.notify();
                                }
                                cx.stop_propagation();
                            } else {
                                this.placement.finish();
                            }
                        })
                        .log_err();
                    }
                });
                window.on_mouse_event({
                    move |event: &gpui::MouseUpEvent, phase, window, cx| {
                        if !phase.capture() || event.button != gpui::MouseButton::Left {
                            return;
                        }
                        weak.update(cx, |this, cx| {
                            if this.placement.is_dragging() {
                                if this
                                    .placement
                                    .update(event.position, window.viewport_size())
                                {
                                    cx.notify();
                                }
                                this.placement.finish();
                                cx.stop_propagation();
                            }
                        })
                        .log_err();
                    }
                });
            },
        )
        .absolute()
        .size_full()
    }
}

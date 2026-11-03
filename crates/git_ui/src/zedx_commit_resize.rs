use super::*;
use gpui::{Bounds, MouseMoveEvent, MouseUpEvent};

#[derive(Default)]
pub(super) struct MessageResize {
    height: Option<Pixels>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    dragging: bool,
    focus: Option<FocusHandle>,
}

impl MessageResize {
    fn height(&self, fallback: Pixels) -> Pixels {
        let height = self
            .bounds
            .get()
            .map_or(fallback, |bounds| bounds.size.height);
        let maximum = (height - px(160.0)).max(px(0.0));
        self.height
            .unwrap_or(height * 0.38)
            .clamp(px(170.0).min(maximum), maximum)
    }

    fn update(&mut self, pointer: Point<Pixels>) {
        if let Some(bounds) = self.bounds.get() {
            self.height = Some(bounds.bottom() - pointer.y - px(2.5));
            self.height = Some(self.height(bounds.size.height));
        }
    }
}

impl GitPanel {
    pub(super) fn prepare_zedx_commit_message(&mut self, cx: &mut Context<Self>) {
        self.zedx_commit
            .message
            .focus
            .get_or_insert_with(|| cx.focus_handle());
        if !self.commit_editor_collapsed
            && !matches!(self.commit_editor.read(cx).mode(), EditorMode::Full { .. })
        {
            self.commit_editor.update(cx, |editor, _| {
                editor.set_mode(EditorMode::Full {
                    scale_ui_elements_with_buffer_font_size: false,
                    show_active_line_background: false,
                    sizing_behavior: SizingBehavior::ExcludeOverscrollMargin,
                })
            });
        }
    }

    pub(super) fn zedx_commit_message_height(&self, window: &Window) -> Pixels {
        if self.commit_editor_collapsed {
            px(80.0)
        } else {
            self.zedx_commit
                .message
                .height(window.viewport_size().height)
        }
    }

    pub(super) fn render_zedx_commit_resize(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("zedx-commit-message-resize")
            .debug_selector(|| "zedx-commit-message-resize".into())
            .w_full()
            .h(px(5.0))
            .flex_none()
            .cursor_row_resize()
            .border_t_1()
            .border_color(cx.theme().colors().border)
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .focusable()
            .when_some(self.zedx_commit.message.focus.clone(), |divider, focus| {
                divider.track_focus(&focus)
            })
            .tab_index(0isize)
            .aria_label("Resize commit message")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if let Some(focus) = &this.zedx_commit.message.focus {
                        focus.focus(window, cx);
                    }
                    this.commit_editor_collapsed = false;
                    if event.click_count >= 2 {
                        this.zedx_commit.message.height = None;
                        this.zedx_commit.message.dragging = false;
                    } else {
                        this.zedx_commit.message.dragging = true;
                    }
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                let modifiers = event.keystroke.modifiers;
                if modifiers.platform || modifiers.control || modifiers.alt || modifiers.function {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "home" => this.zedx_commit.message.height = None,
                    "up" | "down" => {
                        let Some(bounds) = this.zedx_commit.message.bounds.get() else {
                            return;
                        };
                        let delta = if event.keystroke.key == "up" {
                            px(16.0)
                        } else {
                            -px(16.0)
                        };
                        this.zedx_commit.message.height =
                            Some(this.zedx_commit.message.height(bounds.size.height) + delta);
                    }
                    _ => return,
                }
                this.commit_editor_collapsed = false;
                cx.notify();
                cx.stop_propagation();
            }))
    }

    pub(super) fn zedx_commit_resize_tracking(&self, cx: &Context<Self>) -> impl IntoElement {
        let bounds = self.zedx_commit.message.bounds.clone();
        let weak = cx.weak_entity();
        let resize = weak.clone();
        gpui::canvas(
            move |current_bounds, window, cx| {
                if bounds.replace(Some(current_bounds)) != Some(current_bounds) {
                    window.defer(cx, move |_, cx| {
                        weak.update(cx, |_, cx| cx.notify()).log_err();
                    });
                }
            },
            move |_, _, window, _| {
                window.on_mouse_event({
                    let resize = resize.clone();
                    move |event: &MouseMoveEvent, phase, _, cx| {
                        if !phase.capture() {
                            return;
                        }
                        resize
                            .update(cx, |this, cx| {
                                if !this.zedx_commit.message.dragging {
                                    return;
                                }
                                if event.pressed_button == Some(MouseButton::Left) {
                                    this.zedx_commit.message.update(event.position);
                                    cx.stop_propagation();
                                } else {
                                    this.zedx_commit.message.dragging = false;
                                }
                                cx.notify();
                            })
                            .log_err();
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if !phase.capture() || event.button != MouseButton::Left {
                        return;
                    }
                    resize
                        .update(cx, |this, cx| {
                            if this.zedx_commit.message.dragging {
                                this.zedx_commit.message.update(event.position);
                                this.zedx_commit.message.dragging = false;
                                cx.notify();
                                cx.stop_propagation();
                            }
                        })
                        .log_err();
                });
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }
}

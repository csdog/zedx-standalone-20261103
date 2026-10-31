use gpui::{
    Bounds, Hsla, IntoElement, ParentElement, PathBuilder, Pixels, Styled, canvas, div, point, px,
};
use gpui_util::ResultExt;

pub(super) fn corners(radius: Pixels, mut frame: Hsla) -> impl IntoElement {
    // GPUI content masks are rectangular; panel children otherwise paint over the rounded background.
    frame.a = 1.0;
    div()
        .absolute()
        .inset_0()
        .child(
            canvas(
                |_, _, _| (),
                move |bounds: Bounds<Pixels>, _, window, _| {
                    let radius = radius
                        .min(bounds.size.width / 2.0)
                        .min(bounds.size.height / 2.0);
                    if radius <= px(0.0) {
                        return;
                    }
                    let control = radius * (1.0 - 0.552_284_8);
                    for (origin, horizontal, vertical) in [
                        (bounds.origin, 1.0, 1.0),
                        (point(bounds.right(), bounds.top()), -1.0, 1.0),
                        (point(bounds.left(), bounds.bottom()), 1.0, -1.0),
                        (point(bounds.right(), bounds.bottom()), -1.0, -1.0),
                    ] {
                        let mut path = PathBuilder::fill();
                        path.move_to(origin);
                        path.line_to(point(origin.x + radius * horizontal, origin.y));
                        path.cubic_bezier_to(
                            point(origin.x, origin.y + radius * vertical),
                            point(origin.x + control * horizontal, origin.y),
                            point(origin.x, origin.y + control * vertical),
                        );
                        path.close();
                        if let Some(path) = path.build().log_err() {
                            window.paint_path(path, frame);
                        }
                    }
                },
            )
            .size_full(),
        )
        .size_full()
        .min_w(px(0.0))
        .min_h(px(0.0))
}

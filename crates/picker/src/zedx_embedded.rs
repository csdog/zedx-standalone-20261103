use crate::{ElementContainer, Picker, PickerDelegate, Presentation, preview::Layout, shape};
use gpui::{Context, Pixels, Rems, Window, px};

impl<D: PickerDelegate> Picker<D> {
    /// Fits the fork's embedded Search Everywhere without changing standalone picker sizing.
    pub fn fit_zedx_search(
        &mut self,
        width: Pixels,
        available_height: Pixels,
        results_height: Pixels,
        window: &Window,
    ) {
        if !matches!(self.presentation, Presentation::Embedded) {
            return;
        }
        let visible = self
            .preview
            .as_ref()
            .is_some_and(|preview| preview.layout != Layout::Hidden);
        if visible && let Some(preview) = &mut self.preview {
            preview.layout = Layout::Below;
        }
        let desired_preview = if window.viewport_size().width < px(760.0) {
            px(175.0)
        } else {
            px(205.0)
        };
        // Preserve a useful result list when a short window needs a smaller preview.
        let desired_preview =
            desired_preview.min((available_height - results_height.min(px(220.0))).max(px(0.0)));
        let height = (results_height + if visible { desired_preview } else { px(0.0) })
            .min(available_height)
            .max(px(1.0));
        let preview_height = if visible {
            desired_preview.min((height - px(100.0)).max(px(0.0)))
        } else {
            px(0.0)
        };
        // A hidden preview uses height as the list cap; its input and footer are outside that cap.
        let height = if visible {
            height
        } else {
            (height - px(75.0)).max(px(1.0))
        };
        let width = width.max(px(1.0));
        self.size_bounds.max_width = shape::RelativeWidth::from_pixels(width, window);
        self.size_bounds.max_height = shape::RelativeHeight::from_pixels(height, window);
        self.size_bounds.min_results.width = Rems(width.min(px(280.0)) / window.rem_size());
        self.size_bounds.min_results.height =
            Rems((height - preview_height).min(px(90.0)) / window.rem_size());
        self.size_bounds.min_preview.height =
            Rems(preview_height.min(px(60.0)) / window.rem_size());
        self.shape = shape::Shape::HorizontallyCentered(shape::Centered {
            width: shape::RelativeWidth::from_pixels(width, window),
            height: shape::RelativeHeight::from_pixels(height, window),
            preview_size: shape::ViewportFraction::from_height_pixels(preview_height, window),
        });
    }
    /// Refreshes streamed rows while preserving the producer task and scroll position.
    pub fn zedx_streamed_matches_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let ElementContainer::List(state) = &mut self.element_container {
            let offset = state.logical_scroll_top();
            state.reset(self.delegate.match_count());
            state.scroll_to(offset);
        }
        if let Some(update) = self.delegate.try_get_preview_data_for_match(cx)
            && let Some(preview) = &mut self.preview
        {
            preview.update(update, window, cx);
        }
        cx.notify();
    }
}

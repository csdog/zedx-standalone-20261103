use cocoa::{
    appkit::{NSView, NSWindow},
    base::{id, nil},
};
use gpui::Pixels;
use objc::{
    msg_send,
    runtime::{NO, YES},
    sel, sel_impl,
};

// The caller keeps the NSWindow alive and invokes this on its main UI thread.
pub(super) unsafe fn set_content_corner_radius(window: id, radius: Pixels) {
    unsafe {
        let content = window.contentView();
        if content == nil {
            return;
        }
        // The frame also paints the native outline. Clip it with the blur and Metal content
        // so a smaller system outline cannot remain outside the requested corner.
        let frame: id = msg_send![content, superview];
        let clip_view = if frame == nil { content } else { frame };
        clip_view.setWantsLayer(YES);
        let layer: id = msg_send![clip_view, layer];
        if layer == nil {
            return;
        }
        let radius = radius.to_f64().max(0.0);
        let current_radius: f64 = msg_send![layer, cornerRadius];
        let masks_to_bounds: objc::runtime::BOOL = msg_send![layer, masksToBounds];
        let clipping = if radius > 0.0 { YES } else { NO };
        if current_radius != radius || masks_to_bounds != clipping {
            let _: () = msg_send![layer, setCornerRadius: radius];
            let _: () = msg_send![layer, setMasksToBounds: clipping];
        }
    }
}

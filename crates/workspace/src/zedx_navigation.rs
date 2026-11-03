use super::*;

impl Workspace {
    pub(super) fn open_zedx_thread_history(
        &mut self,
        _: &zedx_actions::ShowThreadHistory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = window.root::<MultiWorkspace>().flatten() else {
            return;
        };
        window.defer(cx, move |window, cx| {
            root.update(cx, |root, cx| {
                if !root.multi_workspace_enabled(cx) {
                    return;
                }
                root.open_sidebar(cx);
                if let Some(sidebar) = root.sidebar() {
                    sidebar.show_thread_history(window, cx);
                }
            });
        });
    }
}

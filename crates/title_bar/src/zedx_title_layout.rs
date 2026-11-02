use super::*;

pub(super) fn enabled() -> bool {
    std::env::var_os("ZEDX_UPSTREAM_UI").is_none()
}

pub(super) fn content_height(window: &Window) -> gpui::Pixels {
    px(if window.viewport_size().width <= px(760.0) {
        38.0
    } else {
        40.0
    })
}

pub(super) fn control_size(_window: &Window) -> gpui::Size<gpui::Pixels> {
    gpui::size(px(32.0), px(34.0))
}

impl TitleBar {
    pub(super) fn configure_zedx_title_layout(&self, window: &Window, cx: &mut Context<Self>) {
        if enabled() {
            window.set_content_corner_radius(px(if window.is_fullscreen() { 0.0 } else { 13.0 }));
        }
        let height = enabled().then(|| content_height(window));
        let changed = self
            .platform_titlebar
            .update(cx, |titlebar, _| titlebar.set_content_height(height));
        if changed && cfg!(target_os = "macos") {
            let height = height.unwrap_or_else(|| platform_title_bar_height(window));
            window.set_traffic_light_position(gpui::point(px(12.0), (height - px(14.0)) / 2.0));
        }
    }

    pub(super) fn render_zedx_branch(
        &self,
        repository: Entity<project::git_store::Repository>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !TitleBarSettings::get_global(cx).show_branch_name {
            return None;
        }
        let workspace = self.workspace.clone();
        workspace.upgrade()?;
        let name = repository
            .read(cx)
            .branch
            .as_ref()
            .map(|branch| branch.name().to_string())
            .unwrap_or_else(|| "Detached HEAD".into());
        let label = util::truncate_and_trailoff(&name, 28);
        Some(
            PopoverMenu::new("zedx-branch-menu")
                .menu(move |window, cx| {
                    git_ui_core::build_branch_picker(
                        workspace.clone(),
                        Some(repository.clone()),
                        window,
                        cx,
                    )
                })
                .trigger_with_tooltip(
                    ui::ButtonLike::new("zedx-branch")
                        .height(px(30.0).into())
                        .tab_index(0isize)
                        .aria_label("Git branch")
                        .child(
                            h_flex()
                                .gap(px(6.0))
                                .px(px(7.0))
                                .max_w(px(if window.viewport_size().width <= px(1100.0) {
                                    120.0
                                } else {
                                    210.0
                                }))
                                .min_w_0()
                                .child(ui::zedx::icon(ui::zedx::Icon::Git, cx).size(px(16.0)))
                                .child(
                                    Label::new(label)
                                        .size(ui::zedx::label_size())
                                        .color(Color::Muted)
                                        .truncate(),
                                )
                                .child(
                                    Icon::new(IconName::ChevronDown)
                                        .size(IconSize::XSmall)
                                        .color(Color::Muted),
                                ),
                        ),
                    Tooltip::text(format!("Git branch · {name}")),
                )
                .anchor(gpui::Anchor::TopLeft)
                .into_any_element(),
        )
    }
}

#[path = "zedx_island.rs"]
mod island;
pub fn island_corners(radius: gpui::Pixels, frame: gpui::Hsla) -> impl gpui::IntoElement {
    island::corners(radius, frame)
}

pub fn material_corners(radius: gpui::Pixels, frame: gpui::Hsla) -> impl gpui::IntoElement {
    island::material_corners(radius, frame)
}

use crate::{ButtonCommon, ButtonLike, FixedWidth, Label, LabelCommon, LabelSize, Tooltip};
use gpui::{App, ElementId, Img, InteractiveElement, ParentElement, SharedString, Styled, img, px};
use theme::ActiveTheme;

#[derive(Clone, Copy, strum::AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum Icon {
    Project,
    FindTool,
    Commit,
    Git,
    Structure,
    Terminal,
    Play,
    Debug,
    Settings,
    Folder,
    Library,
    Scratches,
    Chevron,
    Down,
    More,
    Minus,
    Check,
    Spark,
    Database,
    Bell,
    Split,
    Refresh,
    Sun,
    ArrowUp,
    Code,
    History,
    Warning,
    RunTool,
    FileRust,
    FileToml,
    FileMarkdown,
    FileCargo,
    Collapse,
    Close,
    Menu,
    Search,
    SearchPreview,
    SearchFilter,
    SearchFind,
    SearchClass,
    SearchHistory,
}

pub fn asset(path: SharedString) -> Img {
    img(path).flex_none()
}

pub fn icon(icon: Icon, cx: &App) -> Img {
    let appearance = if cx.theme().appearance().is_light() {
        "light"
    } else {
        "dark"
    };
    asset(format!("icons/zedx/{}-{appearance}.svg", icon.as_ref()).into()).size(px(20.0))
}

/// Selected tool windows share one opaque accent across both rails.
pub fn tool_window_selected_background() -> gpui::Hsla {
    gpui::rgb(0x3574f0).into()
}

pub fn tool_window_selected_icon(glyph: Icon, cx: &App) -> gpui::Svg {
    let appearance = if cx.theme().appearance().is_light() {
        "light"
    } else {
        "dark"
    };
    gpui::svg()
        .path(format!("icons/zedx/{}-{appearance}.svg", glyph.as_ref()))
        .size(px(20.0))
        .text_color(gpui::white())
}

pub fn tool_button(
    id: impl Into<ElementId>,
    glyph: Icon,
    label: &'static str,
    cx: &App,
) -> ButtonLike {
    ButtonLike::new(id)
        .width(px(24.0))
        .height(px(24.0).into())
        .tab_index(0isize)
        .aria_label(label)
        .tooltip(Tooltip::text(label))
        .child(icon(glyph, cx).size(px(16.0)))
}

/// Primary ZedX labels follow the configured UI font size without an extra small-label multiplier.
pub fn label_size() -> LabelSize {
    LabelSize::Custom(gpui::rems(1.0))
}

/// Keep secondary metadata one step below primary labels while retaining UI scaling.
pub fn caption_size() -> LabelSize {
    LabelSize::Custom(gpui::rems(12.0 / 13.0))
}

/// The prototype's `.tool-window-head`: one icon, one title, a bottom border, and room for a
/// caller-supplied meta label and hide control on the right.
pub fn tool_window_head(
    id: &'static str,
    glyph: Icon,
    title: impl Into<SharedString>,
    cx: &App,
) -> gpui::Div {
    crate::h_flex()
        .debug_selector(move || format!("zedx-head-{id}"))
        .h(px(35.0))
        .flex_none()
        .px_2()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().colors().border)
        .child(icon(glyph, cx).size(px(16.0)))
        .child(
            Label::new(title)
                .size(label_size())
                .weight(gpui::FontWeight::MEDIUM),
        )
}

pub fn tool_window_icon(name: &str) -> Option<Icon> {
    Some(match name {
        "Find" => Icon::FindTool,
        "Project Panel" => Icon::Project,
        "TerminalPanel" => Icon::Terminal,
        "GitPanel" => Icon::Commit,
        "Git history" => Icon::Git,
        "Outline Panel" | "Structure" => Icon::Structure,
        "Problems" => Icon::Warning,
        "Run" => Icon::RunTool,
        "DebugPanel" => Icon::Debug,
        "AgentPanel" => Icon::Spark,
        _ => return None,
    })
}

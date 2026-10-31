#[path = "zedx_island.rs"]
mod island;
pub fn island_corners(radius: gpui::Pixels, frame: gpui::Hsla) -> impl gpui::IntoElement {
    island::corners(radius, frame)
}

use crate::{ButtonCommon, ButtonLike, FixedWidth, Tooltip};
use gpui::{App, ElementId, Img, ParentElement, SharedString, Styled, img, px};
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

pub fn tool_window_icon(name: &str) -> Option<Icon> {
    Some(match name {
        "Find" => Icon::FindTool,
        "Project Panel" => Icon::Project,
        "TerminalPanel" => Icon::Terminal,
        "GitPanel" => Icon::Commit,
        "Outline Panel" | "Structure" => Icon::Structure,
        "Problems" => Icon::Warning,
        "Run" => Icon::RunTool,
        "DebugPanel" => Icon::Debug,
        "AgentPanel" => Icon::Spark,
        _ => return None,
    })
}

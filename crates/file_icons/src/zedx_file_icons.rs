use super::*;
use theme::ActiveTheme;

pub(super) fn asset(name: &str, cx: &App) -> Option<SharedString> {
    if !cx.theme().name.starts_with("ZedX ") {
        return None;
    }
    let appearance = if cx.theme().appearance().is_light() {
        "light"
    } else {
        "dark"
    };
    Some(format!("icons/zedx/{name}-{appearance}.svg").into())
}

pub(super) fn for_path(path: &Path, cx: &App) -> Option<SharedString> {
    let name = match path.file_name()?.to_str()? {
        "Cargo.toml" | "Cargo.lock" => "file-cargo",
        file if file.ends_with(".rs") => "file-rust",
        file if file.ends_with(".toml") => "file-toml",
        file if file.ends_with(".md") || file.ends_with(".markdown") => "file-markdown",
        _ => return None,
    };
    asset(name, cx)
}

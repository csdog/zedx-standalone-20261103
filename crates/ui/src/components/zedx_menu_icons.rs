#[path = "zedx_menu_outline.rs"]
mod outline;
use crate::IconName;
pub(super) use outline::place_icons_first;
pub use outline::zedx_menu_icon;

/// Common menu symbols; all icon names are also accepted in settings JSON.
pub const ZEDX_MENU_ICONS: &[(&str, &str, IconName)] = &[
    ("copy", "Copy", IconName::Copy),
    ("scissors", "Cut", IconName::Scissors),
    ("zedx_paste", "Paste", IconName::ZedxPaste),
    ("file", "File", IconName::File),
    ("folder_add", "New Folder", IconName::FolderAdd),
    ("folder_open", "Open Folder", IconName::FolderOpen),
    ("pencil", "Rename", IconName::Pencil),
    ("trash", "Delete", IconName::Trash),
    ("undo", "Undo", IconName::Undo),
    ("rotate_cw", "Redo", IconName::RotateCw),
    ("clock", "History", IconName::Clock),
    ("terminal", "Terminal", IconName::Terminal),
    ("magnifying_glass", "Search", IconName::MagnifyingGlass),
    ("link", "Link", IconName::Link),
    ("code", "Code", IconName::Code),
    ("arrow_right", "Navigate", IconName::ArrowRight),
    ("git_commit", "Commit", IconName::GitCommit),
    ("git_branch", "Branch", IconName::GitBranch),
    ("diff", "Diff", IconName::Diff),
    ("check", "Check", IconName::Check),
];

pub fn zedx_context_menu_icon(id: &str, label: &str, configured: Option<&str>) -> Option<IconName> {
    if let Some(name) = configured {
        return name.parse().ok();
    }
    let command = id
        .rsplit("::")
        .next()
        .unwrap_or(id)
        .split('#')
        .next()
        .unwrap_or(id);
    match command {
        "Copy" | "Duplicate" => return Some(IconName::Copy),
        "Cut" => return Some(IconName::Scissors),
        "Paste" | "PasteText" => return Some(IconName::ZedxPaste),
        "Undo" => return Some(IconName::Undo),
        "Redo" => return Some(IconName::RotateCw),
        "Rename" | "RenameFile" => return Some(IconName::Pencil),
        "NewFile" => return Some(IconName::File),
        "NewDirectory" | "NewFolder" => return Some(IconName::FolderAdd),
        "Trash" | "Delete" | "Clear" => return Some(IconName::Trash),
        "StageAll" => return Some(IconName::Plus),
        "UnstageAll" | "RestoreFile" => return Some(IconName::Undo),
        "ToggleStaged" => return Some(IconName::Check),
        _ => {}
    }
    match label.trim().trim_end_matches('…') {
        "Copy" | "Duplicate" => Some(IconName::Copy),
        "Cut" => Some(IconName::Scissors),
        "Paste" => Some(IconName::ZedxPaste),
        "Undo" | "Restore File" => Some(IconName::Undo),
        "Redo" => Some(IconName::RotateCw),
        "Rename" | "Rename Symbol" => Some(IconName::Pencil),
        "Trash" | "Delete" => Some(IconName::Trash),
        "New File" => Some(IconName::File),
        "New Folder" => Some(IconName::FolderAdd),
        "Reveal in Finder" | "Open in Default App" => Some(IconName::FolderOpen),
        "Open in Terminal" => Some(IconName::Terminal),
        "Copy Path" | "Copy Relative Path" | "Copy File Permalink" | "Copy Permalink to Line" => {
            Some(IconName::Link)
        }
        "View History" | "View File History" => Some(IconName::Clock),
        "Go to Definition"
        | "Go to Declaration"
        | "Go to Implementation"
        | "Go to Type Definition" => Some(IconName::ArrowRight),
        "Find All References" => Some(IconName::MagnifyingGlass),
        "Format Buffer" | "Format Selections" => Some(IconName::Code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_support_actions_and_callback_labels() {
        assert_eq!(
            zedx_context_menu_icon("editor::Copy", "Copy", None),
            Some(IconName::Copy)
        );
        assert_eq!(
            zedx_context_menu_icon("label:Open in Terminal", "Open in Terminal", None),
            Some(IconName::Terminal)
        );
        assert_eq!(zedx_context_menu_icon("custom:1", "Unknown", None), None);
    }
    #[test]
    fn explicit_icon_and_hidden_icon_override_defaults() {
        assert_eq!(
            zedx_context_menu_icon("editor::Copy", "Copy", Some("pencil")),
            Some(IconName::Pencil)
        );
        assert_eq!(
            zedx_context_menu_icon("editor::Copy", "Copy", Some("")),
            None
        );
        for (name, _, icon) in ZEDX_MENU_ICONS {
            assert_eq!(name.parse::<IconName>().ok(), Some(*icon));
        }
    }
}

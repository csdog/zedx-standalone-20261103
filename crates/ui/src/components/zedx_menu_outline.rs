use super::*;

pub fn zedx_menu_icon(icon: IconName) -> crate::Icon {
    outline_path(icon).map_or_else(|| crate::Icon::new(icon), crate::Icon::from_path)
}

fn outline_path(icon: IconName) -> Option<gpui::SharedString> {
    if !matches!(
        icon,
        IconName::Copy
            | IconName::Scissors
            | IconName::ZedxPaste
            | IconName::File
            | IconName::Folder
            | IconName::FolderOpen
            | IconName::FolderAdd
            | IconName::Pencil
            | IconName::Trash
            | IconName::Undo
            | IconName::RotateCcw
            | IconName::RotateCw
            | IconName::Clock
            | IconName::Terminal
            | IconName::MagnifyingGlass
            | IconName::Link
            | IconName::Code
            | IconName::ArrowRight
            | IconName::GitCommit
            | IconName::GitBranch
            | IconName::Diff
            | IconName::Check
            | IconName::Plus
            | IconName::ZedxRun
            | IconName::PlayOutlined
            | IconName::PlayFilled
            | IconName::Debug
    ) {
        return None;
    }
    let name: &'static str = icon.into();
    Some(format!("icons/zedx_menu/{name}.svg").into())
}

pub(crate) fn place_icons_first(items: &mut [crate::ContextMenuItem]) {
    for item in items {
        if let crate::ContextMenuItem::Entry(entry) = item {
            entry.icon_position = crate::IconPosition::Start;
            entry.icon_size = crate::IconSize::Medium;
            if let Some(path) = entry.icon.and_then(outline_path) {
                entry.icon = None;
                entry.custom_icon_path = Some(path);
                entry.custom_icon_svg = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trailing_icons_move_to_the_front_and_use_outline_geometry() {
        let mut items = vec![
            crate::ContextMenuEntry::new("Copy")
                .icon(IconName::Copy)
                .icon_position(crate::IconPosition::End)
                .into(),
        ];
        place_icons_first(&mut items);
        let crate::ContextMenuItem::Entry(entry) = &items[0] else {
            panic!("entry")
        };
        assert!(matches!(entry.icon_position, crate::IconPosition::Start));
        assert!(
            entry
                .custom_icon_path
                .as_ref()
                .is_some_and(|path| path.starts_with("icons/zedx_menu/"))
        );
    }

    #[test]
    fn outline_normalization_preserves_run_green() {
        let green = gpui::rgb(0x208a3c).into();
        let mut items = vec![
            crate::ContextMenuEntry::new("Run")
                .icon(IconName::ZedxRun)
                .icon_color(crate::Color::Custom(green))
                .into(),
        ];
        place_icons_first(&mut items);
        let crate::ContextMenuItem::Entry(entry) = &items[0] else {
            panic!("entry")
        };
        assert!(matches!(entry.icon_color, Some(crate::Color::Custom(color)) if color == green));
        assert_eq!(
            entry.custom_icon_path.as_deref(),
            Some("icons/zedx_menu/zedx_run.svg")
        );
    }

    #[test]
    fn common_menu_icons_have_outline_only_paint() {
        for (_, _, icon) in ZEDX_MENU_ICONS {
            let path = outline_path(*icon).expect("common icon has an outline");
            assert!(path.starts_with("icons/zedx_menu/"));
            assert!(path.ends_with(".svg"));
        }
    }
}

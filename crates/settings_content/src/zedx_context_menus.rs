use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

/// ZedX context menu customizations. Each scope is independent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(default)]
pub struct ZedxContextMenus {
    pub editor: Option<ZedxContextMenu>,
    pub project: Option<ZedxContextMenu>,
    pub git: Option<ZedxContextMenu>,
    pub terminal: Option<ZedxContextMenu>,
}
impl ZedxContextMenus {
    pub fn get(&self, scope: &str) -> Option<&ZedxContextMenu> {
        match scope {
            "editor" => self.editor.as_ref(),
            "project" => self.project.as_ref(),
            "git" => self.git.as_ref(),
            "terminal" => self.terminal.as_ref(),
            _ => None,
        }
    }
    pub fn set(&mut self, scope: &str, value: Option<ZedxContextMenu>) -> bool {
        let target = match scope {
            "editor" => &mut self.editor,
            "project" => &mut self.project,
            "git" => &mut self.git,
            "terminal" => &mut self.terminal,
            _ => return false,
        };
        *target = value;
        true
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(default)]
pub struct ZedxContextMenu {
    /// IDs of items to hide. The Customize dialog displays the available items.
    pub hidden: Vec<String>,
    /// Preferred item order. Unlisted items retain their original relative order.
    pub order: Vec<String>,
    /// Extra registered actions. These run with the original menu target's focus.
    pub items: Vec<ZedxContextMenuAction>,
    /// Item ID to icon name (snake_case). An empty string hides the icon.
    pub icons: collections::HashMap<String, String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(default)]
pub struct ZedxContextMenuAction {
    /// A unique ID for this custom item within the menu.
    pub id: String,
    pub label: String,
    /// Registered command name, e.g. editor::Copy.
    pub action: String,
    /// Optional action arguments, using the same object as keymap action arguments.
    pub arguments: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge_from::MergeFrom as _;

    #[test]
    fn root_settings_deserialize_and_serialize_context_menu_configuration() {
        let settings: crate::SettingsContent = serde_json::from_str(
            r#"{"zedx_context_menus":{"editor":{"hidden":["editor::Copy"]}}}"#,
        )
        .unwrap();
        let menus = settings.zedx_context_menus.as_ref().unwrap();
        assert_eq!(menus.editor.as_ref().unwrap().hidden, ["editor::Copy"]);
        let serialized = serde_json::to_value(&settings).unwrap();
        assert_eq!(
            serialized["zedx_context_menus"]["editor"]["hidden"][0],
            "editor::Copy"
        );
        assert!(crate::SettingsContent::NAMED_DESERIALIZE_KEYS.contains(&"zedx_context_menus"));
    }

    #[test]
    fn root_settings_without_menu_configuration_keep_it_optional() {
        for json in ["{}", r#"{"zedx_context_menus":null}"#] {
            let settings: crate::SettingsContent = serde_json::from_str(json).unwrap();
            assert!(settings.zedx_context_menus.is_none());
        }
    }
    #[test]
    fn icon_configuration_survives_root_settings_round_trip() {
        let settings: crate::SettingsContent = serde_json::from_str(
            r#"{"zedx_context_menus":{"project":{"icons":{"label:Copy":"copy","label:Paste":""}}}}"#,
        ).unwrap();
        let serialized = serde_json::to_string(&settings).unwrap();
        let restored: crate::SettingsContent = serde_json::from_str(&serialized).unwrap();
        let icons = &restored.zedx_context_menus.unwrap().project.unwrap().icons;
        assert_eq!(icons.get("label:Copy").map(String::as_str), Some("copy"));
        assert_eq!(icons.get("label:Paste").map(String::as_str), Some(""));
    }

    #[test]
    fn scopes_merge_independently_and_empty_lists_can_reset_overrides() {
        let mut config: ZedxContextMenus = serde_json::from_str(r#"{"editor":{"hidden":["editor::Copy"]},"project":{"order":["project_panel::NewFile"]}}"#).unwrap();
        config.merge_from(&serde_json::from_str(r#"{"editor":{"hidden":[]}}"#).unwrap());
        assert!(config.editor.unwrap().hidden.is_empty());
        assert_eq!(config.project.unwrap().order, ["project_panel::NewFile"]);
    }
    #[test]
    fn custom_actions_accept_arguments_and_unknown_scopes_are_rejected() {
        let item: ZedxContextMenuAction = serde_json::from_str(r#"{"id":"find","label":"Find…","action":"zedx::FindInFiles","arguments":{"query":"needle"}}"#).unwrap();
        assert_eq!(item.arguments.unwrap()["query"], "needle");
        assert!(!ZedxContextMenus::default().set("unknown", None));
    }
}

use crate::Workspace;
use gpui::{App, Context, actions};
use settings::update_settings_file;
use theme::{ActiveTheme, Appearance, SystemAppearance};

actions!(
    zedx,
    [
        SearchEverywhere,
        SearchFiles,
        SearchActions,
        SearchFindInFiles,
        RestoreDefaultLayout,
        ShowDatabase,
        ShowGitHistory,
        ToggleCommit,
        ToggleRightAssistant,
        ToggleRightOutline,
        ShowNotifications,
        ShowThreadHistory,
        ToggleBranches,
        ToggleStructure,
        ToggleProblems,
        ToggleRun,
        NextSearchCategory,
        PreviousSearchCategory,
        ToggleTheme
    ]
);

#[derive(
    Clone, Debug, Default, PartialEq, serde::Deserialize, schemars::JsonSchema, gpui::Action,
)]
#[action(namespace = zedx)]
pub struct MoveToolWindow {
    /// Persistent panel name, e.g. `Structure`.
    pub name: String,
    /// Rail positions to move by; negative is towards the start of the rail.
    pub delta: i32,
}
#[derive(Clone, Default, PartialEq, serde::Deserialize, schemars::JsonSchema, gpui::Action)]
#[action(namespace = zedx)]
pub struct FindInFiles {
    pub query: Option<String>,
    pub include_ignored: Option<bool>,
}
#[derive(Clone, Default, PartialEq, serde::Deserialize, schemars::JsonSchema, gpui::Action)]
#[action(namespace = zedx)]
pub struct SelectSearchCategory {
    pub index: usize,
}

pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace, window, cx: &mut Context<Workspace>| {
            workspace.register_action(|workspace, action: &MoveToolWindow, window, cx| {
                workspace.move_zedx_tool_window(&action.name, action.delta as isize, window, cx);
            });
            if let Some(window) = window {
                workspace.observe_zedx_compact_docks(window, cx);
            }
            workspace.observe_zedx_right_sidebar(cx);
            workspace.register_action(|workspace, _: &ToggleRightAssistant, window, cx| {
                workspace.toggle_zedx_right_panel("AgentPanel", window, cx)
            });
            workspace.register_action(|workspace, _: &ToggleRightOutline, window, cx| {
                workspace.toggle_zedx_right_panel("Outline Panel", window, cx)
            });
            workspace.register_action(Workspace::restore_zedx_layout);
            workspace.register_action(Workspace::open_zedx_thread_history);
            workspace.register_action(|workspace, _: &ShowNotifications, _, cx| {
                workspace.toggle_zedx_notification_center(cx);
            });
            workspace.register_action(|workspace, _: &ToggleTheme, _, cx| {
                let appearance = if cx.theme().appearance().is_light() {
                    Appearance::Dark
                } else {
                    Appearance::Light
                };
                let name = if appearance.is_light() {
                    "ZedX Light"
                } else {
                    "ZedX Dark"
                };
                let system_appearance = SystemAppearance::global(cx).0;
                update_settings_file(workspace.app_state().fs.clone(), cx, move |settings, _| {
                    theme_settings::set_theme(settings, name, appearance, system_appearance);
                });
            });
        },
    )
    .detach();
}

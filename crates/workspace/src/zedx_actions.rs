use crate::{Toast, Workspace, notifications::NotificationId};
use gpui::{App, Context, actions};
use settings::update_settings_file;
use theme::{ActiveTheme, Appearance, SystemAppearance};

actions!(
    zedx,
    [
        SearchEverywhere,
        SearchFiles,
        SearchActions,
        RestoreDefaultLayout,
        ShowDatabase,
        ShowNotifications,
        ToggleStructure,
        ToggleProblems,
        ToggleRun,
        NextSearchCategory,
        PreviousSearchCategory,
        ToggleTheme
    ]
);

#[derive(Clone, Default, PartialEq, serde::Deserialize, schemars::JsonSchema, gpui::Action)]
#[action(namespace = zedx)]
pub struct FindInFiles {
    pub query: Option<String>,
    pub include_ignored: Option<bool>,
}

pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace, window, cx: &mut Context<Workspace>| {
            if let Some(window) = window {
                workspace.observe_zedx_compact_docks(window, cx);
            }
            workspace.register_action(Workspace::restore_zedx_layout);
            workspace.register_action(|workspace, _: &ShowDatabase, _, cx| {
                workspace.show_toast(
                    Toast::new(
                        NotificationId::unique::<ShowDatabase>(),
                        "Database tool window is reserved for a future implementation",
                    )
                    .autohide(),
                    cx,
                );
            });
            workspace.register_action(|workspace, _: &ShowNotifications, _, cx| {
                workspace.show_toast(
                    Toast::new(
                        NotificationId::unique::<ShowNotifications>(),
                        "ZedX workbench is ready",
                    )
                    .autohide(),
                    cx,
                );
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

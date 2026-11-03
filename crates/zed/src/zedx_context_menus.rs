#[path = "zedx_context_menu_icons.rs"]
mod icon_picker;
use editor::Editor;
use gpui::{App, Context, Entity, FocusHandle, Focusable, Render, Subscription, Window};
use settings::{Settings, SettingsStore, ZedxContextMenu, ZedxContextMenuAction, ZedxContextMenus};
use ui::{
    Checkbox, CustomizeContextMenu, IconButton, Tooltip, ZedxExtraMenuItem, ZedxMenuItem,
    ZedxMenuRule, ZedxMenus, prelude::*,
};
use util::ResultExt;
use workspace::Workspace;

#[derive(Default)]
struct LatestSettings(ZedxContextMenus);
impl gpui::Global for LatestSettings {}

const SCOPES: [&str; 4] = ["editor", "project", "git", "terminal"];
#[derive(Clone, Debug, settings::RegisterSetting)]
struct MenuSettings(ZedxContextMenus);
impl Settings for MenuSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        Self(content.zedx_context_menus.clone().unwrap_or_default())
    }
}
fn compile_rule(config: &ZedxContextMenu, cx: &App) -> anyhow::Result<ZedxMenuRule> {
    anyhow::ensure!(
        config.items.len() <= 64
            && config.order.len() <= 512
            && config.hidden.len() <= 512
            && config.icons.len() <= 512,
        "Menu customization is limited to 64 extra commands and 512 item IDs per option"
    );
    for (id, icon) in &config.icons {
        anyhow::ensure!(!id.trim().is_empty(), "Menu icon item IDs must be nonempty");
        anyhow::ensure!(
            icon.is_empty() || icon.parse::<IconName>().is_ok(),
            "Unknown menu icon: {icon}"
        );
    }
    let mut ids = std::collections::HashSet::new();
    let mut items = Vec::new();
    for item in &config.items {
        anyhow::ensure!(
            !item.id.trim().is_empty() && ids.insert(item.id.as_str()),
            "Custom command IDs must be nonempty and unique"
        );
        anyhow::ensure!(
            !item.label.trim().is_empty(),
            "Custom commands require a label"
        );
        let action = cx.build_action(&item.action, item.arguments.clone())?;
        items.push(ZedxExtraMenuItem {
            id: item.id.clone(),
            label: item.label.clone(),
            action,
        });
    }
    Ok(ZedxMenuRule {
        hidden: config.hidden.clone(),
        order: config.order.clone(),
        items,
        icons: config
            .icons
            .iter()
            .map(|(id, icon)| (id.clone(), icon.clone()))
            .collect(),
    })
}
fn apply_settings(cx: &mut App) {
    let content = MenuSettings::get_global(cx).0.clone();
    cx.set_global(LatestSettings(content.clone()));
    let mut rules = std::collections::HashMap::new();
    for scope in SCOPES {
        if let Some(config) = content.get(scope) {
            match compile_rule(config, cx) {
                Ok(rule) => {
                    rules.insert(scope.to_string(), rule);
                }
                Err(error) => log::error!("Invalid {scope} context menu settings: {error}"),
            }
        }
    }
    cx.update_global::<ZedxMenus, _>(|menus, _| menus.rules = rules);
    cx.refresh_windows();
}
fn seed_catalogs(cx: &mut App) {
    let scopes: &[(&str, &[(&str, &str)])] = &[
        (
            "editor",
            &[
                ("editor::GoToDefinition", "Go to Definition"),
                ("editor::GoToDeclaration", "Go to Declaration"),
                ("editor::GoToTypeDefinition", "Go to Type Definition"),
                ("editor::GoToImplementation", "Go to Implementation"),
                ("editor::FindAllReferences", "Find All References"),
                ("editor::Rename", "Rename Symbol"),
                ("editor::Format", "Format Buffer"),
                ("editor::Cut", "Cut"),
                ("editor::Copy", "Copy"),
                ("editor::Paste", "Paste"),
            ],
        ),
        (
            "project",
            &[
                ("project_panel::NewFile", "New File"),
                ("project_panel::NewDirectory", "New Folder"),
                ("project_panel::Rename", "Rename"),
                ("project_panel::Copy", "Copy"),
                ("project_panel::Cut", "Cut"),
                ("project_panel::Paste", "Paste"),
            ],
        ),
        (
            "git",
            &[
                ("git::StageAll", "Stage All"),
                ("git::UnstageAll", "Unstage All"),
                ("git::ToggleStaged", "Stage / Unstage"),
                ("git::RestoreFile", "Restore File"),
            ],
        ),
        (
            "terminal",
            &[
                ("terminal::Copy", "Copy"),
                ("terminal::Paste", "Paste"),
                ("terminal::PasteText", "Paste Text"),
                ("terminal::Clear", "Clear"),
            ],
        ),
    ];
    let catalogs = scopes
        .iter()
        .map(|(scope, items)| {
            let entries = items
                .iter()
                .filter_map(|(name, label)| match cx.build_action(name, None) {
                    Ok(action) => Some(ZedxMenuItem {
                        id: action.name().to_string(),
                        label: (*label).into(),
                        separator: false,
                    }),
                    Err(error) => {
                        log::debug!("Skipping default menu item {name}: {error}");
                        None
                    }
                })
                .collect::<Vec<_>>();
            ((*scope).to_string(), entries)
        })
        .collect::<Vec<_>>();
    cx.update_global::<ZedxMenus, _>(|menus, _| {
        for (scope, items) in catalogs {
            menus.catalogs.entry(scope).or_insert(items);
        }
    });
}
pub(super) fn init(cx: &mut App) {
    if std::env::var_os("ZEDX_UPSTREAM_UI").is_some() {
        return;
    }
    if !cx.has_global::<ZedxMenus>() {
        cx.set_global(ZedxMenus::default());
    }
    seed_catalogs(cx);
    apply_settings(cx);
    cx.observe_global::<SettingsStore>(apply_settings).detach();
    settings_ui::register_context_menu_settings_editor(
        |scope, fs, window, cx| {
            cx.new(|cx| Customizer::new(scope.to_string(), fs, window, cx))
                .into()
        },
        cx,
    );
    // Keep old keymaps/action links working by redirecting to the Settings page.
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(|_, action: &CustomizeContextMenu, window, cx| {
            if !SCOPES.contains(&action.scope.as_str()) {
                return;
            }
            settings_ui::select_context_menu_settings_scope(&action.scope, cx);
            window.dispatch_action(
                Box::new(zed_actions::OpenSettingsAt {
                    path: "zedx_context_menus".into(),
                    target: Some(zed_actions::OpenSettingsAtTarget::User),
                }),
                cx,
            );
        });
    })
    .detach();
}
#[derive(Clone)]
struct Row {
    item: ZedxMenuItem,
    visible: bool,
}
struct Customizer {
    scope: String,
    saved_notice: bool,
    fs: std::sync::Arc<dyn fs::Fs>,
    focus: FocusHandle,
    rows: Vec<Row>,
    selected: Option<usize>,
    config: ZedxContextMenu,
    base: Vec<ZedxMenuItem>,
    search: Entity<Editor>,
    label: Entity<Editor>,
    arguments: Entity<Editor>,
    candidate: Option<String>,
    adding: bool,
    pending: bool,
    reset: bool,
    error: Option<String>,
    _search_subscription: Subscription,
}
impl Customizer {
    fn new(
        scope: String,
        fs: std::sync::Arc<dyn fs::Fs>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let config = cx
            .global::<LatestSettings>()
            .0
            .get(&scope)
            .cloned()
            .unwrap_or_default();
        let base = cx
            .global::<ZedxMenus>()
            .catalogs
            .get(&scope)
            .cloned()
            .unwrap_or_default();
        let rows = rows_for(&base, &config);
        let error = compile_rule(&config, cx)
            .err()
            .map(|error| error.to_string());
        let search = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Search commands…", window, cx);
            editor
        });
        let label = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Menu label", window, cx);
            editor
        });
        let arguments = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Optional arguments (JSON object)", window, cx);
            editor
        });
        let observer = cx.subscribe(&search, |this: &mut Self, _, event, cx| {
            if matches!(event, editor::EditorEvent::BufferEdited) {
                this.candidate = None;
                cx.notify();
            }
        });
        Self {
            scope,
            saved_notice: false,
            fs,
            focus: cx.focus_handle(),
            rows,
            selected: None,
            config,
            base,
            search,
            label,
            arguments,
            candidate: None,
            adding: false,
            pending: false,
            reset: false,
            error,
            _search_subscription: observer,
        }
    }
    fn move_selected(&mut self, offset: isize, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        let Some(index) = self.selected else {
            return;
        };
        let target = index
            .saturating_add_signed(offset)
            .min(self.rows.len().saturating_sub(1));
        if index < self.rows.len() && target != index {
            self.rows.swap(index, target);
            self.selected = Some(target);
            self.reset = false;
            self.saved_notice = false;
            cx.notify();
        }
    }
    fn revert_saved(&mut self, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        self.config = cx
            .global::<LatestSettings>()
            .0
            .get(&self.scope)
            .cloned()
            .unwrap_or_default();
        self.rows = rows_for(&self.base, &self.config);
        self.selected = None;
        self.reset = false;
        self.error = None;
        self.saved_notice = false;
        cx.notify();
    }
    fn restore(&mut self, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        self.config = ZedxContextMenu::default();
        self.rows = rows_for(&self.base, &self.config);
        self.selected = None;
        self.reset = true;
        self.saved_notice = false;
        self.error = None;
        cx.notify();
    }
    fn add(&mut self, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        let Some(name) = self.candidate.clone() else {
            return;
        };
        let text = self.arguments.read(cx).text(cx);
        let arguments = if text.trim().is_empty() {
            None
        } else {
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(arguments) => Some(arguments),
                Err(error) => {
                    self.error = Some(format!("Invalid command arguments: {error}"));
                    cx.notify();
                    return;
                }
            }
        };
        if let Err(error) = cx.build_action(&name, arguments.clone()) {
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        if arguments.is_none()
            && let Some(index) = self.rows.iter().position(|row| row.item.id == name)
        {
            self.rows[index].visible = true;
            self.selected = Some(index);
            self.error = None;
            self.adding = false;
            self.reset = false;
            self.saved_notice = false;
            cx.notify();
            return;
        }
        let label = self.label.read(cx).text(cx);
        if label.trim().is_empty() {
            self.error = Some("Enter a menu label.".into());
            cx.notify();
            return;
        }
        if self.config.items.len() >= 64 {
            self.error = Some("A menu supports up to 64 custom commands.".into());
            cx.notify();
            return;
        }
        let Some(id) = (1..=64)
            .map(|index| format!("user-{index}"))
            .find(|id| !self.config.items.iter().any(|item| &item.id == id))
        else {
            self.error = Some("A menu supports up to 64 custom commands.".into());
            cx.notify();
            return;
        };
        self.config.items.push(ZedxContextMenuAction {
            id: id.clone(),
            label: label.clone(),
            action: name,
            arguments,
        });
        self.rows.push(Row {
            item: ZedxMenuItem {
                id: format!("custom:{id}"),
                label,
                separator: false,
            },
            visible: true,
        });
        self.selected = Some(self.rows.len() - 1);
        self.adding = false;
        self.reset = false;
        self.saved_notice = false;
        self.error = None;
        cx.notify();
    }
    fn remove_selected(&mut self, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        let Some(index) = self.selected else {
            return;
        };
        let Some(row) = self.rows.get(index) else {
            return;
        };
        let Some(id) = row.item.id.strip_prefix("custom:") else {
            return;
        };
        self.config.items.retain(|item| item.id != id);
        let removed = self.rows[index].item.id.clone();
        self.config.hidden.retain(|item| item != &removed);
        self.config.order.retain(|item| item != &removed);
        self.config.icons.remove(&removed);
        self.rows.remove(index);
        self.selected = None;
        self.reset = false;
        self.saved_notice = false;
        cx.notify();
    }
    fn export(&self) -> ZedxContextMenu {
        if self.reset {
            return ZedxContextMenu::default();
        }
        let known = self
            .rows
            .iter()
            .map(|row| row.item.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut hidden = self
            .config
            .hidden
            .iter()
            .filter(|id| !known.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        hidden.extend(
            self.rows
                .iter()
                .filter(|row| !row.visible)
                .map(|row| row.item.id.clone()),
        );
        let mut order = self
            .rows
            .iter()
            .map(|row| row.item.id.clone())
            .collect::<Vec<_>>();
        order.extend(
            self.config
                .order
                .iter()
                .filter(|id| !known.contains(id.as_str()))
                .cloned(),
        );
        ZedxContextMenu {
            hidden,
            order,
            items: self.config.items.clone(),
            icons: self.config.icons.clone(),
        }
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        let config = self.export();
        let rule = match compile_rule(&config, cx) {
            Ok(rule) => rule,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        let remembered = config.clone();
        let scope = self.scope.clone();
        let saved_scope = scope.clone();
        let request = settings::update_settings_file_with_completion(
            self.fs.clone(),
            cx,
            move |settings, _| {
                settings
                    .zedx_context_menus
                    .get_or_insert_with(Default::default)
                    .set(&saved_scope, Some(config));
            },
        );
        self.pending = true;
        self.saved_notice = false;
        self.error = None;
        cx.notify();
        cx.spawn(async move |dialog, cx| {
            let result = request.await.unwrap_or_else(|error| Err(error.into()));
            dialog
                .update(cx, |dialog, cx| {
                    dialog.pending = false;
                    match result {
                        Ok(()) => {
                            cx.update_global::<LatestSettings, _>(|latest, _| {
                                latest.0.set(&scope, Some(remembered));
                            });
                            cx.update_global::<ZedxMenus, _>(|menus, _| {
                                menus.rules.insert(scope, rule);
                            });
                            dialog.config = cx
                                .global::<LatestSettings>()
                                .0
                                .get(&dialog.scope)
                                .cloned()
                                .unwrap_or_default();
                            dialog.saved_notice = true;
                            cx.notify();
                        }
                        Err(error) => {
                            dialog.error = Some(format!("Could not save menu settings: {error}"));
                            cx.notify();
                        }
                    }
                })
                .log_err();
        })
        .detach();
    }
}
fn rows_for(base: &[ZedxMenuItem], config: &ZedxContextMenu) -> Vec<Row> {
    let mut rows = base
        .iter()
        .cloned()
        .chain(config.items.iter().map(|item| ZedxMenuItem {
            id: format!("custom:{}", item.id),
            label: item.label.clone(),
            separator: false,
        }))
        .map(|item| Row {
            visible: !config.hidden.contains(&item.id),
            item,
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        config
            .order
            .iter()
            .position(|id| id == &row.item.id)
            .unwrap_or(usize::MAX)
    });
    rows
}
impl Focusable for Customizer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for Customizer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = match self.scope.as_str() {
            "editor" => "Editor",
            "project" => "Project",
            "git" => "Git",
            _ => "Terminal",
        };
        let query = self.search.read(cx).text(cx).to_lowercase();
        let candidates = cx
            .all_action_names()
            .iter()
            .filter_map(|name| {
                let label = command_palette::humanize_action_name(name);
                (label.to_lowercase().contains(&query) || name.to_lowercase().contains(&query))
                    .then_some(((*name).to_string(), label))
            })
            .take(6)
            .collect::<Vec<_>>();
        v_flex()
            .id("zedx-menu-customizer")
            .debug_selector(|| "zedx-menu-customizer".into())
            .w_full()
            .p_3()
            .gap_2()
            .track_focus(&self.focus)
            .key_context("ZedxMenuCustomizer")
            .child(Label::new(format!("{title} Context Menu")).weight(gpui::FontWeight::MEDIUM))
            .child(
                Label::new(
                    "Show or hide items, change their order, choose icons, or add commands.",
                )
                .size(ui::zedx::caption_size())
                .color(Color::Muted),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("menu-add", "Add Command…")
                            .disabled(self.pending)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.adding = !this.adding;
                                if this.adding {
                                    window.focus(&this.search.focus_handle(cx), cx);
                                }
                                cx.notify();
                            })),
                    )
                    .child(self.icon_picker(cx))
                    .child(
                        IconButton::new("menu-item-up", IconName::ArrowUp)
                            .disabled(self.pending || self.selected.is_none_or(|index| index == 0))
                            .tooltip(Tooltip::text("Move Up"))
                            .on_click(cx.listener(|this, _, _, cx| this.move_selected(-1, cx))),
                    )
                    .child(
                        IconButton::new("menu-item-down", IconName::ArrowDown)
                            .disabled(
                                self.pending
                                    || self
                                        .selected
                                        .is_none_or(|index| index + 1 >= self.rows.len()),
                            )
                            .tooltip(Tooltip::text("Move Down"))
                            .on_click(cx.listener(|this, _, _, cx| this.move_selected(1, cx))),
                    )
                    .child(
                        IconButton::new("menu-item-remove", IconName::Trash)
                            .disabled(
                                self.pending
                                    || !self
                                        .selected
                                        .and_then(|index| self.rows.get(index))
                                        .is_some_and(|row| row.item.id.starts_with("custom:")),
                            )
                            .tooltip(Tooltip::text("Remove Custom Command"))
                            .on_click(cx.listener(|this, _, _, cx| this.remove_selected(cx))),
                    ),
            )
            .child(
                v_flex()
                    .id("menu-customization-items")
                    .max_h(px(280.0))
                    .overflow_y_scroll()
                    .flex_shrink_0()
                    .children(self.rows.iter().enumerate().map(|(index, row)| {
                        h_flex()
                            .id(("menu-customization-row", index))
                            .min_h(px(30.0))
                            .px_2()
                            .gap_2()
                            .rounded(px(4.0))
                            .when(self.selected == Some(index), |row| {
                                row.bg(cx.theme().colors().element_selected)
                            })
                            .child(
                                Checkbox::new(
                                    ("menu-item-visible", index),
                                    if row.visible {
                                        ToggleState::Selected
                                    } else {
                                        ToggleState::Unselected
                                    },
                                )
                                .disabled(self.pending)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if let Some(row) = this.rows.get_mut(index) {
                                            row.visible = !row.visible;
                                            this.reset = false;
                                            this.saved_notice = false;
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                div().w(px(20.0)).flex_none().children(
                                    (!row.item.separator)
                                        .then(|| {
                                            ui::zedx_context_menu_icon(
                                                &row.item.id,
                                                &row.item.label,
                                                self.config
                                                    .icons
                                                    .get(&row.item.id)
                                                    .map(String::as_str),
                                            )
                                        })
                                        .flatten()
                                        .map(|icon| {
                                            ui::zedx_menu_icon(icon)
                                                .size(IconSize::Small)
                                                .color(Color::Muted)
                                        }),
                                ),
                            )
                            .child(
                                Label::new(row.item.label.clone())
                                    .size(ui::zedx::label_size())
                                    .truncate(),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected = Some(index);
                                cx.notify();
                            }))
                    })),
            )
            .when(self.adding, |view| {
                view.child(
                    v_flex()
                        .gap_1()
                        .p_2()
                        .border_1()
                        .rounded_md()
                        .border_color(cx.theme().colors().border)
                        .child(div().h(px(26.0)).child(self.search.clone()))
                        .children(candidates.into_iter().map(|(name, label)| {
                            Button::new(
                                SharedString::from(format!("menu-command-{name}")),
                                label.clone(),
                            )
                            .full_width()
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.candidate = Some(name.clone());
                                    this.label.update(cx, |editor, cx| {
                                        editor.set_text(label.clone(), window, cx)
                                    });
                                    this.arguments
                                        .update(cx, |editor, cx| editor.set_text("", window, cx));
                                    cx.notify();
                                },
                            ))
                        }))
                        .when(self.candidate.is_some(), |view| {
                            view.child(div().h(px(26.0)).child(self.label.clone()))
                                .child(div().h(px(26.0)).child(self.arguments.clone()))
                                .child(
                                    Button::new("menu-confirm-add", "Add")
                                        .on_click(cx.listener(|this, _, _, cx| this.add(cx))),
                                )
                        }),
                )
            })
            .when(self.saved_notice, |view| {
                view.child(Label::new("Saved").color(Color::Muted))
            })
            .when_some(self.error.clone(), |view, error| {
                view.child(Label::new(error).color(Color::Error))
            })
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .child(
                        Button::new("menu-restore-default", "Restore Defaults")
                            .disabled(self.pending)
                            .on_click(cx.listener(|this, _, _, cx| this.restore(cx))),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("menu-customization-cancel", "Revert Changes")
                                    .disabled(self.pending)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.revert_saved(cx);
                                    })),
                            )
                            .child(
                                Button::new(
                                    "menu-customization-save",
                                    if self.pending { "Saving…" } else { "Save" },
                                )
                                .disabled(self.pending)
                                .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
#[path = "zedx_context_menus_tests.rs"]
mod tests;

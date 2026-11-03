use super::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, schemars::JsonSchema, gpui::Action)]
#[action(namespace = zedx)]
#[serde(default)]
pub struct CustomizeContextMenu {
    pub scope: String,
}

impl Default for CustomizeContextMenu {
    fn default() -> Self {
        Self {
            scope: "editor".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ZedxMenuItem {
    pub id: String,
    pub label: String,
    pub separator: bool,
}
pub struct ZedxExtraMenuItem {
    pub id: String,
    pub label: String,
    pub action: Box<dyn Action>,
}
impl Clone for ZedxExtraMenuItem {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            label: self.label.clone(),
            action: self.action.boxed_clone(),
        }
    }
}
#[derive(Clone, Default)]
pub struct ZedxMenuRule {
    pub hidden: Vec<String>,
    pub order: Vec<String>,
    pub items: Vec<ZedxExtraMenuItem>,
    pub icons: HashMap<String, String>,
}
#[derive(Default)]
pub struct ZedxMenus {
    pub rules: HashMap<String, ZedxMenuRule>,
    pub catalogs: HashMap<String, Vec<ZedxMenuItem>>,
}
impl gpui::Global for ZedxMenus {}

pub(super) fn enabled() -> bool {
    std::env::var_os("ZEDX_UPSTREAM_UI").is_none()
}
pub(super) fn surface<T: Styled>(element: T, cx: &App) -> T {
    if !enabled() {
        return element;
    }
    element
        .rounded(px(8.0))
        .border_1()
        .border_color(if cx.theme().appearance().is_light() {
            gpui::rgb(0xd3d5db)
        } else {
            gpui::rgb(0x43454a)
        })
        .bg(if cx.theme().appearance().is_light() {
            gpui::rgb(0xffffff)
        } else {
            gpui::rgb(0x2b2d30)
        })
        .shadow_lg()
}
pub(super) fn row(item: ListItem) -> ListItem {
    if enabled() {
        item.zedx_menu().height(rems(2.0)).rounded()
    } else {
        item
    }
}

impl ContextMenu {
    /// Gives a right-click menu a stable, independently customizable scope.
    pub fn zedx_scope(mut self, scope: impl Into<SharedString>) -> Self {
        self.zedx_scope = Some(scope.into());
        self
    }
    pub(super) fn finish_zedx(mut self, window: &Window, cx: &mut Context<Self>) -> Self {
        if !enabled() {
            return self;
        }
        let Some(scope) = self.zedx_scope.clone() else {
            super::zedx_menu_icons::place_icons_first(&mut self.items);
            return self;
        };
        let rule = cx
            .try_global::<ZedxMenus>()
            .and_then(|menus| menus.rules.get(scope.as_ref()))
            .cloned()
            .unwrap_or_default();
        let original = catalog(&self.items);
        if cx.has_global::<ZedxMenus>() {
            cx.update_global::<ZedxMenus, _>(|menus, _| {
                menus.catalogs.insert(scope.to_string(), original.clone());
            });
        }
        let mut added_ids = Vec::new();
        for extra in &rule.items {
            // An exact duplicate must retain the original callback and disabled
            // state. Different arguments are allowed, but inherit any disabled
            // state for the same command in the original context.
            let duplicate = self.items.iter().any(|item| matches!(item,
                ContextMenuItem::Entry(entry) if entry.action.as_ref().is_some_and(|action| action.partial_eq(&*extra.action))));
            if duplicate {
                continue;
            }
            let guarded = self.items.iter().any(|item| matches!(item,
                ContextMenuItem::Entry(entry) if entry.disabled && entry.action.as_ref().is_some_and(|action| action.name() == extra.action.name())));
            let available = self.action_context.as_ref().map_or_else(
                || window.is_action_available(&*extra.action, cx),
                |focus| window.is_action_available_in(&*extra.action, focus),
            );
            self = self.action_disabled_when(
                guarded || !available,
                extra.label.clone(),
                extra.action.boxed_clone(),
            );
            added_ids.push(format!("custom:{}", extra.id));
        }
        let mut items = std::mem::take(&mut self.items);
        let mut descriptions = catalog(&items);
        let original_len = original.len();
        for (item, id) in descriptions.iter_mut().skip(original_len).zip(added_ids) {
            item.id = id;
        }
        for (item, description) in items.iter_mut().zip(&descriptions) {
            let configured = rule.icons.get(&description.id).map(String::as_str);
            match item {
                ContextMenuItem::Entry(entry) => {
                    if configured.is_some()
                        || (entry.icon.is_none()
                            && entry.custom_icon_path.is_none()
                            && entry.custom_icon_svg.is_none())
                    {
                        entry.icon = super::zedx_menu_icons::zedx_context_menu_icon(
                            &description.id,
                            &description.label,
                            configured,
                        );
                        if configured.is_some() {
                            entry.custom_icon_path = None;
                            entry.custom_icon_svg = None;
                        }
                    }
                }
                ContextMenuItem::Submenu { icon, .. } if configured.is_some() => {
                    *icon = super::zedx_menu_icons::zedx_context_menu_icon(
                        &description.id,
                        &description.label,
                        configured,
                    );
                }
                _ => {}
            }
        }
        let hidden = rule
            .hidden
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut rank = HashMap::new();
        for (index, id) in rule.order.iter().enumerate() {
            rank.entry(id.as_str()).or_insert(index);
        }
        let mut indexed = items
            .drain(..)
            .zip(descriptions)
            .enumerate()
            .filter(|(_, (_, item))| !hidden.contains(item.id.as_str()))
            .collect::<Vec<_>>();
        indexed.sort_by_key(|(index, (_, item))| {
            (
                rank.get(item.id.as_str()).copied().unwrap_or(usize::MAX),
                *index,
            )
        });
        self.items = normalize_separators(indexed.into_iter().map(|(_, (item, _))| item).collect());
        super::zedx_menu_icons::place_icons_first(&mut self.items);

        self
    }
}
fn catalog(items: &[ContextMenuItem]) -> Vec<ZedxMenuItem> {
    let mut ids = HashMap::<String, usize>::new();
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let (base, label, separator) = match item {
                ContextMenuItem::Entry(entry) => (
                    entry
                        .action
                        .as_ref()
                        .map(|action| action.name().to_string())
                        .unwrap_or_else(|| format!("label:{}", entry.label)),
                    entry.label.to_string(),
                    false,
                ),
                ContextMenuItem::Submenu { label, .. } => {
                    (format!("submenu:{label}"), format!("{label} ›"), false)
                }
                ContextMenuItem::Separator => {
                    (format!("separator:{index}"), "────────".into(), true)
                }
                ContextMenuItem::Header(label) | ContextMenuItem::HeaderWithLink(label, ..) => {
                    (format!("header:{index}"), label.to_string(), false)
                }
                ContextMenuItem::Label(label) => {
                    (format!("label:{label}"), label.to_string(), false)
                }
                ContextMenuItem::CustomEntry { .. } => (
                    format!("custom-row:{index}"),
                    "Additional action".into(),
                    false,
                ),
            };
            let count = ids.entry(base.clone()).or_default();
            *count += 1;
            let id = if *count == 1 {
                base
            } else {
                format!("{base}#{}", count)
            };
            ZedxMenuItem {
                id,
                label,
                separator,
            }
        })
        .collect()
}
fn normalize_separators(items: Vec<ContextMenuItem>) -> Vec<ContextMenuItem> {
    let mut output = Vec::new();
    for item in items {
        if matches!(item, ContextMenuItem::Separator)
            && (output.is_empty() || matches!(output.last(), Some(ContextMenuItem::Separator)))
        {
            continue;
        }
        output.push(item);
    }
    if matches!(output.last(), Some(ContextMenuItem::Separator)) {
        output.pop();
    }
    output
}

#[cfg(test)]
#[path = "zedx_context_menu_tests.rs"]
mod tests;

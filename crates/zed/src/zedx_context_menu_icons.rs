use super::*;
use ui::{ContextMenu, ContextMenuEntry, PopoverMenu, ZEDX_MENU_ICONS, zedx_context_menu_icon};

impl Customizer {
    pub(super) fn set_icon(&mut self, id: &str, icon: Option<&str>, cx: &mut Context<Self>) {
        if self.pending
            || !self
                .rows
                .iter()
                .any(|row| row.item.id == id && !row.item.separator)
        {
            return;
        }
        match icon {
            Some(icon) => {
                self.config.icons.insert(id.to_string(), icon.to_string());
            }
            None => {
                self.config.icons.remove(id);
            }
        }
        self.reset = false;
        self.saved_notice = false;
        self.error = None;
        cx.notify();
    }

    pub(super) fn icon_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let row = self
            .selected
            .and_then(|index| self.rows.get(index))
            .filter(|row| !row.item.separator);
        let id = row.map(|row| row.item.id.clone());
        let preview = row.and_then(|row| {
            zedx_context_menu_icon(
                &row.item.id,
                &row.item.label,
                self.config.icons.get(&row.item.id).map(String::as_str),
            )
        });
        let editor = cx.weak_entity();
        PopoverMenu::new("menu-item-icon-picker")
            .trigger(
                Button::new("menu-item-icon", "Icon…")
                    .start_icon(
                        ui::zedx_menu_icon(preview.unwrap_or(IconName::Image))
                            .size(IconSize::Small),
                    )
                    .disabled(self.pending || id.is_none()),
            )
            .menu(move |window, cx| {
                let id = id.clone()?;
                let default_editor = editor.clone();
                let default_id = id.clone();
                let hidden_editor = editor.clone();
                let hidden_id = id.clone();
                let editor = editor.clone();
                Some(ContextMenu::build(window, cx, move |menu, _, _| {
                    let mut menu = menu
                        .entry("Default Icon", None, move |_, cx| {
                            default_editor
                                .update(cx, |this, cx| this.set_icon(&default_id, None, cx))
                                .log_err();
                        })
                        .entry("No Icon", None, move |_, cx| {
                            hidden_editor
                                .update(cx, |this, cx| this.set_icon(&hidden_id, Some(""), cx))
                                .log_err();
                        })
                        .separator();
                    for &(name, label, icon) in ZEDX_MENU_ICONS {
                        let editor = editor.clone();
                        let id = id.clone();
                        menu = menu.item(ContextMenuEntry::new(label).icon(icon).handler(
                            move |_, cx| {
                                editor
                                    .update(cx, |this, cx| this.set_icon(&id, Some(name), cx))
                                    .log_err();
                            },
                        ));
                    }
                    menu
                }))
            })
            .into_any_element()
    }
}

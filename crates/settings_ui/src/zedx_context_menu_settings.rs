use crate::{SettingsPageItem, SettingsWindow, SubPageLink, USER};
use gpui::{AnyView, App, Context, Global, Render, ScrollHandle, Window, prelude::*};
use std::{rc::Rc, sync::Arc};
use ui::prelude::*;

const SCOPES: [(&str, &str); 4] = [
    ("editor", "Editor"),
    ("project", "Project"),
    ("git", "Git"),
    ("terminal", "Terminal"),
];
type EditorBuilder = dyn Fn(&str, Arc<dyn fs::Fs>, &mut Window, &mut App) -> AnyView;
struct EditorFactory {
    build: Rc<EditorBuilder>,
    requested_scope: Option<String>,
}
impl Global for EditorFactory {}

/// Allows the app's owned menu editor to render inside the Settings window.
pub fn register_context_menu_settings_editor(
    build: impl Fn(&str, Arc<dyn fs::Fs>, &mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) {
    cx.set_global(EditorFactory {
        build: Rc::new(build),
        requested_scope: None,
    });
}
pub fn select_context_menu_settings_scope(scope: &str, cx: &mut App) {
    if SCOPES.iter().any(|(name, _)| *name == scope) && cx.has_global::<EditorFactory>() {
        cx.update_global::<EditorFactory, _>(|factory, _| {
            factory.requested_scope = Some(scope.into())
        });
        cx.refresh_windows();
    }
}
pub(crate) fn settings_link() -> SettingsPageItem {
    SettingsPageItem::SubPageLink(SubPageLink {
        title: "Context Menus".into(), r#type: Default::default(),
        description: Some("Choose menu items, change their order, and add commands for Editor, Project, Git and Terminal.".into()),
        search_aliases: &["right click", "context menu", "menus", "右键", "右击", "菜单"],
        json_path: Some("zedx_context_menus"), in_json: true, files: USER,
        render: render_page,
    })
}
fn render_page(
    _: &SettingsWindow,
    scroll: &ScrollHandle,
    window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let view = window.use_keyed_state("zedx-context-menu-settings", cx, |_, _| MenuSettingsPage {
        selected: 0,
        editors: std::array::from_fn(|_| None),
    });
    v_flex()
        .id("zedx-context-menu-settings-page")
        .debug_selector(|| "zedx-context-menu-settings-page".into())
        .size_full()
        .min_h_0()
        .min_w_0()
        .px_8()
        .py_4()
        .overflow_y_scroll()
        .track_scroll(scroll)
        .child(view)
        .into_any_element()
}
struct MenuSettingsPage {
    selected: usize,
    editors: [Option<AnyView>; 4],
}
impl Render for MenuSettingsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(requested) = cx
            .try_global::<EditorFactory>()
            .and_then(|factory| factory.requested_scope.clone())
        {
            if let Some(index) = SCOPES.iter().position(|(scope, _)| *scope == requested) {
                self.selected = index;
            }
            cx.update_global::<EditorFactory, _>(|factory, _| factory.requested_scope = None);
        }
        if self.editors[self.selected].is_none() {
            if let Some(build) = cx
                .try_global::<EditorFactory>()
                .map(|factory| factory.build.clone())
            {
                self.editors[self.selected] = Some(build(
                    SCOPES[self.selected].0,
                    <dyn fs::Fs>::global(cx),
                    window,
                    cx,
                ));
            }
        }
        v_flex()
            .w_full()
            .min_w_0()
            .gap_3()
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(SCOPES.iter().enumerate().map(|(index, (_, label))| {
                        Button::new(("context-menu-settings-scope", index), *label)
                            .toggle_state(index == self.selected)
                            .on_click(cx.listener(move |page, _, _, cx| {
                                page.selected = index;
                                cx.notify();
                            }))
                    })),
            )
            .children(self.editors[self.selected].clone())
            .when(self.editors[self.selected].is_none(), |view| {
                view.child(Label::new("Menu customization is unavailable.").color(Color::Muted))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_link_is_user_scoped_searchable_and_has_the_stable_json_path() {
        let SettingsPageItem::SubPageLink(link) = settings_link() else {
            panic!("expected a settings subpage")
        };
        assert_eq!(link.title.as_ref(), "Context Menus");
        assert_eq!(link.json_path, Some("zedx_context_menus"));
        assert_eq!(link.files, USER);
        assert!(link.search_aliases.contains(&"右击"));
    }
    #[test]
    fn scopes_cover_each_configurable_menu_without_duplicates() {
        let scopes = SCOPES
            .iter()
            .map(|(scope, _)| *scope)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(scopes.len(), 4);
        for scope in ["editor", "project", "git", "terminal"] {
            assert!(scopes.contains(scope));
        }
    }
    struct Fixture(String);
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Label::new(self.0.clone())
        }
    }
    #[gpui::test]
    fn switching_scopes_retains_each_editor_draft(cx: &mut gpui::TestAppContext) {
        let calls = Rc::new(std::cell::Cell::new(0));
        cx.update(|cx| {
            let settings = settings::SettingsStore::test(cx);
            cx.set_global(settings);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            <dyn fs::Fs>::set_global(fs::FakeFs::new(cx.background_executor().clone()), cx);
            register_context_menu_settings_editor(
                {
                    let calls = calls.clone();
                    move |scope, _, _, cx| {
                        calls.set(calls.get() + 1);
                        cx.new(|_| Fixture(scope.into())).into()
                    }
                },
                cx,
            );
        });
        let window = cx.add_window(|_, _| MenuSettingsPage {
            selected: 0,
            editors: std::array::from_fn(|_| None),
        });
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);
        visual.refresh().unwrap();
        let page = window.root(&mut visual).unwrap();
        let first = page.read_with(&visual, |page, _| {
            page.editors[0].as_ref().unwrap().entity_id()
        });
        page.update(&mut visual, |page, cx| {
            page.selected = 1;
            cx.notify();
        });
        visual.refresh().unwrap();
        page.update(&mut visual, |page, cx| {
            page.selected = 0;
            cx.notify();
        });
        visual.refresh().unwrap();
        page.read_with(&visual, |page, _| {
            assert_eq!(page.editors[0].as_ref().unwrap().entity_id(), first)
        });
        assert_eq!(calls.get(), 2);
    }
}

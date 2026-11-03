use super::*;
use gpui::{TestAppContext, VisualTestContext};
gpui::actions!(zedx_menu_tests, [First, Second, Third]);

#[test]
fn separators_are_normalized_without_dropping_entries() {
    let items = normalize_separators(vec![
        ContextMenuItem::Separator,
        ContextMenuItem::Separator,
        ContextMenuItem::Entry(ContextMenuEntry::new("Keep")),
        ContextMenuItem::Separator,
        ContextMenuItem::Separator,
    ]);
    assert_eq!(items.len(), 1);
    assert!(matches!(&items[0], ContextMenuItem::Entry(entry) if entry.label.as_ref() == "Keep"));
}
#[test]
fn identities_distinguish_repeated_actions_and_keep_callbacks_named() {
    let items = vec![
        ContextMenuItem::Entry(ContextMenuEntry::new("One").action(Box::new(First))),
        ContextMenuItem::Entry(ContextMenuEntry::new("Two").action(Box::new(First))),
        ContextMenuItem::Entry(ContextMenuEntry::new("Callback")),
    ];
    let catalog = catalog(&items);
    assert_eq!(catalog[0].id, First.name());
    assert_eq!(catalog[1].id, format!("{}#2", First.name()));
    assert_eq!(catalog[2].id, "label:Callback");
}
fn initialize(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let settings = settings::SettingsStore::test(cx);
        cx.set_global(settings);
        theme_settings::init(theme::LoadThemes::JustBase, cx);
        cx.set_global(ZedxMenus::default());
    });
}
#[gpui::test]
fn customization_keeps_handlers_disabled_flags_and_unlisted_order(cx: &mut TestAppContext) {
    initialize(cx);
    let calls = Rc::new(Cell::new(0));
    cx.update(|cx| {
        cx.update_global::<ZedxMenus, _>(|menus, _| {
            menus.rules.insert(
                "editor".into(),
                ZedxMenuRule {
                    hidden: vec![First.name().into()],
                    order: vec![Third.name().into()],
                    ..Default::default()
                },
            );
        })
    });
    let window = cx.add_window(|window, cx| {
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.zedx_scope("editor")
                .entry("First", Some(Box::new(First)), |_, _| {
                    panic!("hidden item must not execute")
                })
                .separator()
                .item(
                    ContextMenuEntry::new("Second")
                        .action(Box::new(Second))
                        .disabled(true),
                )
                .entry("Third", Some(Box::new(Third)), {
                    let calls = calls.clone();
                    move |_, _| calls.set(calls.get() + 1)
                })
        })
    });
    let menu = window.root(cx).unwrap();
    window.update(cx, |menu, window, cx| {
        let labels = menu.items.iter().filter_map(|item| match item { ContextMenuItem::Entry(entry) => Some(entry.label.as_ref()), _ => None }).collect::<Vec<_>>();
        assert_eq!(labels, ["Third", "Second"]);
        assert!(menu.items.iter().any(|item| matches!(item, ContextMenuItem::Entry(entry) if entry.label.as_ref() == "Second" && entry.disabled)));
        menu.select_first(&SelectFirst, window, cx);
        menu.confirm(&menu::Confirm, window, cx);
    }).unwrap();
    assert_eq!(calls.get(), 1);
    menu.read_with(cx, |_, cx| {
        assert!(
            cx.global::<ZedxMenus>().catalogs["editor"]
                .iter()
                .any(|item| item.id == First.name())
        )
    });
}
#[gpui::test]
fn adding_existing_action_does_not_bypass_disabled_entry(cx: &mut TestAppContext) {
    initialize(cx);
    cx.update(|cx| {
        cx.update_global::<ZedxMenus, _>(|menus, _| {
            menus.rules.insert(
                "git".into(),
                ZedxMenuRule {
                    items: vec![ZedxExtraMenuItem {
                        id: "duplicate".into(),
                        label: "Force enabled?".into(),
                        action: Box::new(First),
                    }],
                    ..Default::default()
                },
            );
        })
    });
    let window = cx.add_window(|window, cx| {
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.zedx_scope("git")
                .action_disabled_when(true, "Original", Box::new(First))
        })
    });
    window
        .read_with(cx, |menu, _| {
            let entries = menu
                .items
                .iter()
                .filter_map(|item| match item {
                    ContextMenuItem::Entry(entry)
                        if entry
                            .action
                            .as_ref()
                            .is_some_and(|action| action.name() == First.name()) =>
                    {
                        Some(entry)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), 1);
            assert!(entries[0].disabled);
        })
        .unwrap();
}
#[gpui::test]
fn idea_menu_rows_follow_ui_font_size_and_have_compact_bounds(cx: &mut TestAppContext) {
    initialize(cx);
    let window = cx.add_window(|window, cx| {
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.entry("One", None, |_, _| {})
                .separator()
                .entry("Two", None, |_, _| {})
        })
    });
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.refresh().unwrap();
    let menu = visual.debug_bounds("zedx-context-menu").unwrap();
    let first = visual.debug_bounds("zedx-context-menu-row-0").unwrap();
    let second = visual.debug_bounds("zedx-context-menu-row-2").unwrap();
    let expected = visual.update(|_, cx| theme::theme_settings(cx).ui_font_size(cx) * 2.0);
    assert_eq!(first.size.height, expected);
    assert_eq!(second.top() - first.bottom(), px(9.0));
    assert!(menu.size.width >= px(220.0));
}

#[test]
fn dynamic_commit_headers_keep_stable_identity() {
    let first = catalog(&[ContextMenuItem::Header("Commit abc123".into())]);
    let next = catalog(&[ContextMenuItem::Header("Commit def456".into())]);
    assert_eq!(first[0].id, next[0].id);
}
#[derive(
    Clone, Debug, Default, PartialEq, serde::Deserialize, schemars::JsonSchema, gpui::Action,
)]
#[action(namespace = zedx_menu_tests)]
struct WithArguments {
    value: usize,
}
#[gpui::test]
fn parameterized_copy_inherits_the_original_command_guard(cx: &mut TestAppContext) {
    initialize(cx);
    cx.update(|cx| {
        cx.update_global::<ZedxMenus, _>(|menus, _| {
            menus.rules.insert(
                "editor".into(),
                ZedxMenuRule {
                    items: vec![ZedxExtraMenuItem {
                        id: "with-args".into(),
                        label: "Other arguments".into(),
                        action: Box::new(WithArguments { value: 2 }),
                    }],
                    ..Default::default()
                },
            );
        })
    });
    let window = cx.add_window(|window, cx| {
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.zedx_scope("editor").action_disabled_when(
                true,
                "Original",
                Box::new(WithArguments { value: 1 }),
            )
        })
    });
    window
        .read_with(cx, |menu, _| {
            let added = menu
                .items
                .iter()
                .find_map(|item| match item {
                    ContextMenuItem::Entry(entry) if entry.label.as_ref() == "Other arguments" => {
                        Some(entry)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(added.disabled);
        })
        .unwrap();
}

#[gpui::test]
fn context_menu_has_no_settings_or_customize_footer(cx: &mut TestAppContext) {
    initialize(cx);
    let window = cx.add_window(|window, cx| {
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.zedx_scope("editor").entry("Copy", None, |_, _| {})
        })
    });
    window.read_with(cx, |menu, _| {
        assert_eq!(menu.items.len(), 1);
        assert!(matches!(&menu.items[0], ContextMenuItem::Entry(entry) if entry.label.as_ref() == "Copy"));
    }).unwrap();
}

#[gpui::test]
fn idea_menu_renders_selected_and_disabled_rows_without_duplicate_hover_styles(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    let window = cx.add_window(|window, cx| {
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.entry("Enabled", None, |_, _| {}).action_disabled_when(
                true,
                "Disabled",
                Box::new(First),
            )
        })
    });
    window
        .update(cx, |menu, _, _| menu.selected_index = Some(0))
        .unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.refresh().unwrap();
    assert!(visual.debug_bounds("zedx-context-menu-row-0").is_some());
    assert!(visual.debug_bounds("zedx-context-menu-row-1").is_some());
}

#[gpui::test]
fn menu_icons_can_be_overridden_hidden_and_reset_without_replacing_handlers(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    let calls = Rc::new(Cell::new(0));
    cx.update(|cx| {
        cx.update_global::<ZedxMenus, _>(|menus, _| {
            menus.rules.insert(
                "project".into(),
                ZedxMenuRule {
                    icons: [
                        ("label:Copy".into(), "pencil".into()),
                        ("label:Paste".into(), "".into()),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                },
            );
        });
    });
    let window = cx.add_window(|window, cx| {
        let calls = calls.clone();
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.zedx_scope("project")
                .entry("Copy", None, move |_, _| calls.set(calls.get() + 1))
                .entry("Paste", None, |_, _| {})
                .entry("Cut", None, |_, _| {})
        })
    });
    window
        .update(cx, |menu, window, cx| {
            let ContextMenuItem::Entry(copy) = &menu.items[0] else {
                panic!("copy entry")
            };
            assert!(copy.custom_icon_path.is_some());
            assert!(matches!(copy.icon_position, IconPosition::Start));
            (copy.handler)(None, window, cx);
            let ContextMenuItem::Entry(paste) = &menu.items[1] else {
                panic!("paste entry")
            };
            assert_eq!(paste.icon, None);
            assert!(paste.custom_icon_svg.is_none());
            let ContextMenuItem::Entry(cut) = &menu.items[2] else {
                panic!("cut entry")
            };
            assert!(cut.custom_icon_path.is_some());
        })
        .unwrap();
    assert_eq!(calls.get(), 1);
    cx.update(|cx| cx.update_global::<ZedxMenus, _>(|menus, _| menus.rules.clear()));
    let reset = cx.add_window(|window, cx| {
        ContextMenu::new(window, cx, |menu, _, _| {
            menu.zedx_scope("project").entry("Copy", None, |_, _| {})
        })
    });
    reset.read_with(cx, |menu, _| {
        assert!(matches!(&menu.items[0], ContextMenuItem::Entry(entry) if entry.custom_icon_path.is_some()));
    }).unwrap();
}

use super::*;
use gpui::{Action as _, TestAppContext};
use settings::RootUserSettings as _;
gpui::actions!(zedx_customizer_tests, [AddedCommand]);
fn base_items() -> Vec<ZedxMenuItem> {
    vec![
        ZedxMenuItem {
            id: "first".into(),
            label: "First".into(),
            separator: false,
        },
        ZedxMenuItem {
            id: "separator:1".into(),
            label: "────────".into(),
            separator: true,
        },
        ZedxMenuItem {
            id: "second".into(),
            label: "Second".into(),
            separator: false,
        },
    ]
}
#[test]
fn configured_order_and_visibility_keep_unlisted_items() {
    let config = ZedxContextMenu {
        hidden: vec!["first".into()],
        order: vec!["second".into()],
        ..Default::default()
    };
    let rows = rows_for(&base_items(), &config);
    assert_eq!(rows[0].item.id, "second");
    assert!(!rows[1].visible);
}
#[gpui::test]
fn invalid_actions_and_duplicate_custom_ids_are_rejected(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut config = ZedxContextMenu {
            items: vec![ZedxContextMenuAction {
                id: "added".into(),
                label: "Added".into(),
                action: AddedCommand.name().into(),
                arguments: None,
            }],
            ..Default::default()
        };
        assert_eq!(compile_rule(&config, cx).unwrap().items.len(), 1);
        config.items.push(config.items[0].clone());
        assert!(compile_rule(&config, cx).is_err());
        config.items.truncate(1);
        config.items[0].action = "missing::NotRegistered".into();
        assert!(compile_rule(&config, cx).is_err());
    });
}
async fn open_editor(
    cx: &mut TestAppContext,
) -> (
    Entity<Workspace>,
    Entity<Customizer>,
    &mut gpui::VisualTestContext,
) {
    let (workspace, cx) = crate::zedx_search::tests::setup(cx).await;
    cx.update(|_, cx| {
        cx.update_global::<ZedxMenus, _>(|menus, _| {
            menus.catalogs.insert("editor".into(), base_items());
        })
    });
    let editor = workspace.update_in(cx, |workspace, window, cx| {
        let fs = workspace.app_state().fs.clone();
        cx.new(|cx| Customizer::new("editor".into(), fs, window, cx))
    });
    (workspace, editor, cx)
}
#[gpui::test]
async fn revert_and_restore_edit_only_draft_without_opening_modal(cx: &mut TestAppContext) {
    let (workspace, editor, cx) = open_editor(cx).await;
    editor.update(cx, |editor, cx| {
        editor.rows[0].visible = false;
        editor.selected = Some(0);
        editor.move_selected(1, cx);
        assert!(editor.export().hidden.contains(&"first".into()));
        editor.revert_saved(cx);
        assert!(editor.rows[0].visible);
        assert!(!editor.pending);
        editor.restore(cx);
        assert!(editor.export().hidden.is_empty());
        assert!(editor.export().order.is_empty());
    });
    workspace.read_with(cx, |_, cx| {
        assert!(MenuSettings::get_global(cx).0.editor.is_none());
    });
}
#[gpui::test]
async fn adding_a_command_and_restoring_defaults_still_work_inside_settings(
    cx: &mut TestAppContext,
) {
    let (_, editor, cx) = open_editor(cx).await;
    editor.update_in(cx, |editor, window, cx| {
        editor.candidate = Some(AddedCommand.name().into());
        editor
            .label
            .update(cx, |label, cx| label.set_text("My command", window, cx));
        editor.add(cx);
        assert_eq!(editor.export().items[0].label, "My command");
        editor.restore(cx);
        assert!(editor.export().items.is_empty());
    });
}
#[gpui::test]
async fn save_preserves_other_scopes_and_keeps_settings_editor_alive(cx: &mut TestAppContext) {
    let (workspace, editor, cx) = open_editor(cx).await;
    let fs = workspace.read_with(cx, |workspace, _| workspace.app_state().fs.clone());
    let file = paths::settings_file();
    let mut tree = serde_json::Map::new();
    tree.insert(
        file.file_name().unwrap().to_str().unwrap().into(),
        serde_json::Value::String(
            "{\"zedx_context_menus\":{\"git\":{\"hidden\":[\"keep-git\"]}}}".into(),
        ),
    );
    fs.as_fake()
        .insert_tree(file.parent().unwrap(), tree.into())
        .await;
    editor.update(cx, |editor, cx| {
        editor.rows[0].visible = false;
        editor.save(cx);
    });
    cx.run_until_parked();
    let contents = fs.load(file).await.unwrap();
    let stored = settings::SettingsContent::parse_json_with_comments(&contents)
        .unwrap()
        .zedx_context_menus
        .unwrap();
    assert_eq!(stored.editor.unwrap().hidden, ["first"]);
    assert_eq!(stored.git.unwrap().hidden, ["keep-git"]);
    editor.update(cx, |editor, cx| {
        assert!(editor.saved_notice);
        assert!(!editor.pending);
        editor.rows[0].visible = true;
        editor.revert_saved(cx);
        assert!(
            !editor.rows[0].visible,
            "revert returns to the last saved settings"
        );
    });
}
#[gpui::test]
async fn malformed_arguments_do_not_mutate_settings(cx: &mut TestAppContext) {
    let (_, editor, cx) = open_editor(cx).await;
    editor.update_in(cx, |editor, window, cx| {
        let count = editor.rows.len();
        editor.candidate = Some(AddedCommand.name().into());
        editor
            .arguments
            .update(cx, |arguments, cx| arguments.set_text("{", window, cx));
        editor.add(cx);
        assert_eq!(editor.rows.len(), count);
        assert!(editor.error.is_some());
        assert!(cx.global::<LatestSettings>().0.editor.is_none());
    });
}
#[gpui::test]
async fn catalog_is_available_before_any_right_click(cx: &mut TestAppContext) {
    let (_, cx) = crate::zedx_search::tests::setup(cx).await;
    cx.update(|_, cx| {
        let menus = cx.global::<ZedxMenus>();
        assert!(
            menus.catalogs["editor"]
                .iter()
                .any(|item| item.id == "editor::Copy")
        );
        for scope in SCOPES {
            assert!(!menus.catalogs[scope].is_empty(), "{scope}");
        }
    });
}
#[gpui::test]
async fn old_customize_action_routes_to_settings_without_opening_a_popup(cx: &mut TestAppContext) {
    let (workspace, cx) = crate::zedx_search::tests::setup(cx).await;
    let routed = std::rc::Rc::new(std::cell::RefCell::new(None));
    workspace.update(cx, |workspace, cx| {
        workspace.register_action({
            let routed = routed.clone();
            move |_, action: &zed_actions::OpenSettingsAt, _, _| {
                *routed.borrow_mut() = Some(action.path.clone())
            }
        });
        cx.notify();
    });
    cx.refresh().unwrap();
    cx.dispatch_action(CustomizeContextMenu {
        scope: "terminal".into(),
    });
    cx.run_until_parked();
    assert_eq!(routed.borrow().as_deref(), Some("zedx_context_menus"));
}

#[gpui::test]
fn menu_icon_names_are_validated_before_saving(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut config = ZedxContextMenu::default();
        config.icons.insert("editor::Copy".into(), "pencil".into());
        config.icons.insert("editor::Paste".into(), "".into());
        let compiled = compile_rule(&config, cx).unwrap();
        assert_eq!(
            compiled.icons.get("editor::Copy").map(String::as_str),
            Some("pencil")
        );
        assert_eq!(
            compiled.icons.get("editor::Paste").map(String::as_str),
            Some("")
        );
        config
            .icons
            .insert("editor::Cut".into(), "not_an_icon".into());
        assert!(compile_rule(&config, cx).is_err());
    });
}
#[gpui::test]
async fn icon_drafts_export_hide_and_restore(cx: &mut TestAppContext) {
    let (_, editor, cx) = open_editor(cx).await;
    editor.update(cx, |this, cx| {
        this.set_icon("first", Some("pencil"), cx);
        assert_eq!(
            this.export().icons.get("first").map(String::as_str),
            Some("pencil")
        );
        this.set_icon("first", Some(""), cx);
        assert_eq!(
            this.export().icons.get("first").map(String::as_str),
            Some("")
        );
        this.set_icon("first", None, cx);
        assert!(!this.export().icons.contains_key("first"));
        this.set_icon("first", Some("copy"), cx);
        this.restore(cx);
        assert!(this.export().icons.is_empty());
    });
}

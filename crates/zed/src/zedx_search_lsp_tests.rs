use super::*;
use futures::StreamExt;
use gpui::{TestAppContext, VisualTestContext};
use language::{FakeLspAdapter, rust_lang};
use std::path::Path;
use util::path;

async fn setup_lsp(
    cx: &mut TestAppContext,
    failing_secondary: bool,
) -> (
    Entity<Workspace>,
    &mut VisualTestContext,
    Vec<lsp::FakeLanguageServer>,
) {
    let (workspace, cx) = tests::setup(cx).await;
    let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
    let fs = project.read_with(cx, |project, _| project.fs().clone());
    fs.create_dir(Path::new(path!("/dependency")))
        .await
        .unwrap();
    fs.atomic_write(
        path!("/dependency/lib.rs").into(),
        "// dependency\npub struct ExternalThing;\npub fn external_fn() {}\n".into(),
    )
    .await
    .unwrap();
    let languages = project.read_with(cx, |project, _| {
        project.languages().add(rust_lang());
        project.languages().clone()
    });
    let mut servers = languages.register_fake_lsp("Rust", FakeLspAdapter {
        capabilities: lsp::ServerCapabilities {
            workspace_symbol_provider: Some(lsp::OneOf::Left(true)),
            ..Default::default()
        },
        initializer: Some(Box::new(|server| {
            server.set_request_handler::<lsp::request::WorkspaceSymbolRequest, _, _>(|params, _| async move {
                if params.query == "fail" {
                    anyhow::bail!("fixture symbol lookup failed");
                }
                #[expect(deprecated, reason = "The LSP flat symbol response includes a deprecated compatibility field")]
                let symbols = [
                    ("LocalThing", path!("/project/main.rs"), lsp::SymbolKind::STRUCT, 0),
                    ("ExternalThing", path!("/dependency/lib.rs"), lsp::SymbolKind::STRUCT, 1),
                    ("external_fn", path!("/dependency/lib.rs"), lsp::SymbolKind::FUNCTION, 2),
                ].into_iter().map(|(name, path, kind, row)| lsp::SymbolInformation {
                    name: name.into(), kind, tags: None, deprecated: None,
                    location: lsp::Location {
                        uri: lsp::Uri::from_file_path(path).unwrap(),
                        range: lsp::Range::new(lsp::Position::new(row, 0), lsp::Position::new(row, 10)),
                    },
                    container_name: None,
                }).collect();
                Ok(Some(lsp::WorkspaceSymbolResponse::Flat(symbols)))
            });
        })),
        ..Default::default()
    });
    let mut secondary_servers = failing_secondary.then(|| {
        languages.register_fake_lsp(
            "Rust",
            FakeLspAdapter {
                name: "failing-secondary",
                capabilities: lsp::ServerCapabilities {
                    workspace_symbol_provider: Some(lsp::OneOf::Left(true)),
                    ..Default::default()
                },
                initializer: Some(Box::new(|server| {
                    server.set_request_handler::<lsp::request::WorkspaceSymbolRequest, _, _>(
                        |_, _| async { anyhow::bail!("secondary symbol lookup failed") },
                    );
                })),
                ..Default::default()
            },
        )
    });
    workspace
        .update_in(cx, |workspace, window, cx| {
            workspace.open_abs_path(
                path!("/project/main.rs").into(),
                workspace::OpenOptions::default(),
                window,
                cx,
            )
        })
        .await
        .unwrap();
    let mut running_servers = vec![servers.next().await.unwrap()];
    if let Some(servers) = secondary_servers.as_mut() {
        running_servers.push(servers.next().await.unwrap());
    }
    cx.run_until_parked();
    (workspace, cx, running_servers)
}

fn search(
    workspace: &Entity<Workspace>,
    category: Category,
    query: &str,
    cx: &mut VisualTestContext,
) -> Entity<SearchWindow> {
    workspace.update_in(cx, |workspace, window, cx| {
        open_category(workspace, Some(category), window, cx)
    });
    let modal = workspace
        .read_with(cx, |workspace, cx| {
            workspace.active_modal::<SearchWindow>(cx)
        })
        .unwrap();
    modal.update_in(cx, |modal, window, cx| {
        modal
            .picker
            .update(cx, |picker, cx| picker.set_query(query, window, cx));
    });
    cx.run_until_parked();
    modal
}

#[gpui::test]
async fn zedx_lsp_external_scope_and_split_navigation(cx: &mut TestAppContext) {
    let (workspace, cx, _server) = setup_lsp(cx, false).await;
    let modal = search(&workspace, Category::Classes, "Thing", cx);
    modal.read_with(cx, |modal, cx| {
        let rows = &modal.picker.read(cx).delegate.rows;
        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            ["LocalThing"]
        );
    });
    cx.simulate_keystrokes("alt-n");
    cx.run_until_parked();
    modal.read_with(cx, |modal, cx| {
        let rows = &modal.picker.read(cx).delegate.rows;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.label == "ExternalThing"
            && row.source == "External"
            && row.detail.ends_with("lib.rs:2")));
    });
    let modal = search(&workspace, Category::Symbols, "Thing", cx);
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            assert_eq!(picker.delegate.rows.len(), 3);
            picker.delegate.selected = picker
                .delegate
                .rows
                .iter()
                .position(|row| row.label == "ExternalThing")
                .unwrap();
            picker.delegate.confirm(true, window, cx);
        });
    });
    cx.run_until_parked();
    workspace.read_with(cx, |workspace, cx| {
        assert_eq!(workspace.panes().len(), 2);
        let editor = workspace.active_item_as::<Editor>(cx).unwrap();
        let editor = editor.read(cx);
        let buffer = editor.buffer().read(cx).as_singleton().unwrap();
        assert_eq!(
            buffer.read(cx).text(),
            "// dependency\npub struct ExternalThing;\npub fn external_fn() {}\n"
        );
    });
    let modal = search(&workspace, Category::Symbols, "Thing", cx);
    // Opening an external result creates a hidden worktree. Project scope must
    // still exclude its symbols after their location changes to InProject.
    modal.update_in(cx, |modal, window, cx| {
        modal.picker.update(cx, |picker, cx| {
            picker.delegate.include_non_project = false;
            let task = picker.delegate.update_matches("Thing".into(), window, cx);
            task.detach();
        });
    });
    cx.run_until_parked();
    modal.read_with(cx, |modal, cx| {
        let rows = &modal.picker.read(cx).delegate.rows;
        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            ["LocalThing"]
        );
    });
}

#[gpui::test]
async fn zedx_lsp_failure_is_shown_and_clears_on_recovery(cx: &mut TestAppContext) {
    let (workspace, cx, _server) = setup_lsp(cx, false).await;
    let modal = search(&workspace, Category::Symbols, "fail", cx);
    modal.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert!(delegate.rows.is_empty());
        assert!(
            delegate
                .error
                .as_ref()
                .is_some_and(|error| error.contains("fixture symbol lookup failed")),
            "actual LSP failure must reach the search footer: {:?}",
            delegate.error
        );
    });
    for width in [1280.0, 390.0] {
        cx.simulate_resize(gpui::size(px(width), px(700.0)));
        cx.run_until_parked();
        cx.refresh().unwrap();
        let status = cx.debug_bounds("zedx-search-status").unwrap();
        let footer = cx.debug_bounds("zedx-search-footer").unwrap();
        assert!(status.size.width > px(0.0));
        assert!(status.left() >= footer.left() && status.right() <= footer.right());
        assert!(status.top() >= footer.top() && status.bottom() <= footer.bottom());
    }
    let modal = search(&workspace, Category::Symbols, "Thing", cx);
    modal.read_with(cx, |modal, cx| {
        let delegate = &modal.picker.read(cx).delegate;
        assert!(delegate.error.is_none());
        assert_eq!(delegate.rows.len(), 1);
    });
}

#[gpui::test]
async fn zedx_lsp_partial_success_survives_other_server_failure(cx: &mut TestAppContext) {
    let (workspace, cx, _servers) = setup_lsp(cx, true).await;
    let project = workspace.read_with(cx, |workspace, _| workspace.project().clone());
    let symbols = project
        .update(cx, |project, cx| project.symbols_with_errors("Thing", cx))
        .await
        .unwrap();
    assert_eq!(
        symbols.len(),
        3,
        "one failing server must not discard successful results"
    );
    let symbols = project
        .update(cx, |project, cx| project.symbols("fail", cx))
        .await
        .unwrap();
    assert!(
        symbols.is_empty(),
        "the upstream API retains its previous behavior"
    );
    let result = project
        .update(cx, |project, cx| project.symbols_with_errors("fail", cx))
        .await;
    assert!(
        result.is_err(),
        "the reporting API must distinguish total failure from no matches"
    );
}

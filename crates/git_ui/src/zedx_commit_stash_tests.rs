fn zedx_stash_entries() -> Vec<git::stash::StashEntry> {
    (0..3)
        .map(|index| git::stash::StashEntry {
            index,
            oid: Oid::from_bytes(&[index as u8 + 1; 20]).unwrap(),
            message: format!("Saved change {index}"),
            branch: Some("main".into()),
            timestamp: 1_700_000_000 + index as i64,
        })
        .collect()
}

async fn setup_zedx_stashes(
    cx: &mut TestAppContext,
) -> (
    Arc<FakeFs>,
    Entity<Workspace>,
    Entity<GitPanel>,
    VisualTestContext,
) {
    init_test(cx);
    let fs = FakeFs::new(cx.background_executor.clone());
    fs.insert_tree(
        path!("/project"),
        json!({".git":{},"a.txt":"keep local changes\n"}),
    )
    .await;
    fs.set_status_for_repo(
        Path::new(path!("/project/.git")),
        &[("a.txt", StatusCode::Modified.worktree())],
    );
    fs.with_git_state(Path::new(path!("/project/.git")), true, |state| {
        state.stash_entries.entries = zedx_stash_entries().into();
    })
    .unwrap();
    fs.insert_tree(
        path!("/other"),
        json!({".git":{}, "other.txt":"other repository\n"}),
    )
    .await;
    fs.with_git_state(Path::new(path!("/other/.git")), true, |state| {
        state.stash_entries.entries = zedx_stash_entries().into();
    })
    .unwrap();
    let project = Project::test(
        fs.clone(),
        [Path::new(path!("/project")), Path::new(path!("/other"))],
        cx,
    )
    .await;
    let repository = project.read_with(cx, |project, cx| {
        project
            .git_store()
            .read(cx)
            .repositories()
            .values()
            .find(|repo| {
                repo.read(cx).work_directory_abs_path.as_ref() == Path::new(path!("/project"))
            })
            .unwrap()
            .clone()
    });
    repository.update(cx, |repository, cx| repository.set_as_active_repository(cx));
    let handle = cx.add_window(|window, cx| MultiWorkspace::test_new(project.clone(), window, cx));
    let workspace = handle
        .read_with(cx, |root, _| root.workspace().clone())
        .unwrap();
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    cx.read(|cx| {
        project
            .read(cx)
            .worktrees(cx)
            .next()
            .unwrap()
            .read(cx)
            .as_local()
            .unwrap()
            .scan_complete()
    })
    .await;
    cx.run_until_parked();
    let panel = workspace.update_in(&mut cx, GitPanel::new);
    await_git_panel_entries(&panel, &mut cx).await;
    panel.read_with(&cx, |panel, cx| {
        assert_eq!(
            panel
                .active_repository
                .as_ref()
                .unwrap()
                .read(cx)
                .cached_stash()
                .entries
                .len(),
            3
        )
    });
    workspace.update_in(&mut cx, |workspace, window, cx| {
        workspace.add_panel(panel.clone(), window, cx);
        workspace.open_panel::<GitPanel>(window, cx);
    });
    panel.update_in(&mut cx, |panel, window, cx| {
        panel.select_zedx_commit_tab(super::zedx_commit_panel::CommitTab::Stash, window, cx);
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    (fs, workspace, panel, cx)
}

fn select_stash_row(cx: &mut VisualTestContext, index: usize) {
    let selector: &'static str = match index {
        0 => "zedx-stash-row-0101010101010101010101010101010101010101",
        1 => "zedx-stash-row-0202020202020202020202020202020202020202",
        _ => "zedx-stash-row-0303030303030303030303030303030303030303",
    };
    let row = cx.debug_bounds(selector).unwrap();
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    cx.refresh().unwrap();
}

#[gpui::test]
async fn test_zedx_stash_drop_cancel_and_reindexed_selection(cx: &mut TestAppContext) {
    let (fs, _, panel, mut cx) = setup_zedx_stashes(cx).await;
    let toolbar = cx.debug_bounds("zedx-stash-toolbar").unwrap();
    assert_eq!(toolbar.size.height, px(30.0));
    select_stash_row(&mut cx, 1);
    let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
    cx.simulate_click(drop.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    assert!(cx.pending_prompt().unwrap().1.contains("Saved change 1"));
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(
        panel.read_with(&cx, |panel, _| panel.stash_entries.entries.len()),
        3
    );
    cx.refresh().unwrap();
    let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
    cx.simulate_click(drop.center(), Modifiers::none());
    cx.run_until_parked();
    fs.with_git_state(Path::new(path!("/project/.git")), true, |state| {
        let mut entries = zedx_stash_entries();
        entries.insert(
            0,
            git::stash::StashEntry {
                index: 0,
                oid: Oid::from_bytes(&[9; 20]).unwrap(),
                message: "Newer stash".into(),
                branch: None,
                timestamp: 1_700_000_100,
            },
        );
        for (index, entry) in entries.iter_mut().enumerate() {
            entry.index = index;
        }
        state.stash_entries.entries = entries.into();
    })
    .unwrap();
    cx.run_until_parked();
    cx.simulate_prompt_answer("Drop Stash");
    cx.run_until_parked();
    await_git_panel_entries(&panel, &mut cx).await;
    panel.read_with(&cx, |panel, _| {
        let entries = &panel.stash_entries.entries;
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "Newer stash");
        assert!(
            entries
                .iter()
                .all(|entry| entry.oid != zedx_stash_entries()[1].oid)
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.oid == zedx_stash_entries()[0].oid)
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.oid == zedx_stash_entries()[2].oid)
        );
    });
    assert_eq!(
        fs.load(Path::new(path!("/project/a.txt"))).await.unwrap(),
        "keep local changes\n"
    );
}

#[gpui::test]
async fn test_zedx_stash_drop_failure_retains_entry_and_allows_retry(cx: &mut TestAppContext) {
    let (fs, _, panel, mut cx) = setup_zedx_stashes(cx).await;
    select_stash_row(&mut cx, 0);
    fs.with_git_state(Path::new(path!("/project/.git")), false, |state| {
        state.simulated_stash_drop_error = Some("Cannot lock stash reference".into());
    })
    .unwrap();
    let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
    cx.simulate_click(drop.center(), Modifiers::none());
    cx.run_until_parked();
    cx.simulate_prompt_answer("Drop Stash");
    cx.run_until_parked();
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-stash-error").is_some());
    assert_eq!(
        panel.read_with(&cx, |panel, _| panel.stash_entries.entries.len()),
        3
    );
    fs.with_git_state(Path::new(path!("/project/.git")), false, |state| {
        state.simulated_stash_drop_error = None;
    })
    .unwrap();
    let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
    cx.simulate_click(drop.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Drop Stash");
    cx.run_until_parked();
    await_git_panel_entries(&panel, &mut cx).await;
    assert_eq!(
        panel.read_with(&cx, |panel, _| panel.stash_entries.entries.len()),
        2
    );
}

#[gpui::test]
async fn test_zedx_stash_keyboard_wraps_and_enter_previews_without_applying(
    cx: &mut TestAppContext,
) {
    let (fs, workspace, panel, mut cx) = setup_zedx_stashes(cx).await;
    let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
    cx.simulate_click(drop.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    let apply = cx.debug_bounds("zedx-stash-apply").unwrap();
    cx.simulate_click(apply.center(), Modifiers::none());
    cx.run_until_parked();
    for (previous, expected) in [
        (false, "Saved change 0"),
        (true, "Saved change 2"),
        (false, "Saved change 0"),
    ] {
        if previous {
            cx.dispatch_action(menu::SelectPrevious);
        } else {
            cx.dispatch_action(menu::SelectNext);
        }
        cx.run_until_parked();
        cx.refresh().unwrap();
        let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
        cx.simulate_click(drop.center(), Modifiers::none());
        cx.run_until_parked();
        assert!(cx.pending_prompt().unwrap().1.contains(expected));
        cx.simulate_prompt_answer("Cancel");
        cx.run_until_parked();
    }
    cx.dispatch_action(menu::Confirm);
    cx.run_until_parked();
    workspace.read_with(&cx, |workspace, cx| {
        assert!(workspace.active_item_as::<CommitView>(cx).is_some());
    });
    assert_eq!(
        panel.read_with(&cx, |panel, _| panel.stash_entries.entries.len()),
        3
    );
    assert_eq!(
        fs.load(Path::new(path!("/project/a.txt"))).await.unwrap(),
        "keep local changes\n"
    );
}

#[gpui::test]
async fn test_zedx_stash_removed_during_confirmation_does_not_drop_another_entry(
    cx: &mut TestAppContext,
) {
    let (fs, _, panel, mut cx) = setup_zedx_stashes(cx).await;
    select_stash_row(&mut cx, 1);
    let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
    cx.simulate_click(drop.center(), Modifiers::none());
    cx.run_until_parked();
    fs.with_git_state(Path::new(path!("/project/.git")), true, |state| {
        let mut entries = zedx_stash_entries();
        entries.remove(1);
        for (index, entry) in entries.iter_mut().enumerate() {
            entry.index = index;
        }
        state.stash_entries.entries = entries.into();
    })
    .unwrap();
    cx.run_until_parked();
    cx.simulate_prompt_answer("Drop Stash");
    cx.run_until_parked();
    await_git_panel_entries(&panel, &mut cx).await;
    cx.refresh().unwrap();
    assert!(cx.debug_bounds("zedx-stash-error").is_some());
    panel.read_with(&cx, |panel, _| {
        assert_eq!(panel.stash_entries.entries.len(), 2);
        assert_eq!(
            panel.stash_entries.entries[0].oid,
            zedx_stash_entries()[0].oid
        );
        assert_eq!(
            panel.stash_entries.entries[1].oid,
            zedx_stash_entries()[2].oid
        );
    });
}

#[gpui::test]
async fn test_zedx_stash_repository_change_cancels_pending_delete(cx: &mut TestAppContext) {
    let (fs, _, panel, mut cx) = setup_zedx_stashes(cx).await;
    select_stash_row(&mut cx, 0);
    let drop = cx.debug_bounds("zedx-stash-drop").unwrap();
    cx.simulate_click(drop.center(), Modifiers::none());
    cx.run_until_parked();
    let other = panel.read_with(&cx, |panel, cx| {
        panel
            .project
            .read(cx)
            .git_store()
            .read(cx)
            .repositories()
            .values()
            .find(|repo| {
                repo.read(cx).work_directory_abs_path.as_ref() == Path::new(path!("/other"))
            })
            .unwrap()
            .clone()
    });
    other.update(&mut cx, |repo, cx| repo.set_as_active_repository(cx));
    cx.simulate_prompt_answer("Drop Stash");
    cx.run_until_parked();
    await_git_panel_entries(&panel, &mut cx).await;
    cx.refresh().unwrap();
    panel.read_with(&cx, |panel, _| {
        assert_eq!(
            panel.active_repository.as_ref().unwrap().entity_id(),
            other.entity_id()
        );
        assert_eq!(panel.stash_entries.entries.len(), 3);
    });
    fs.with_git_state(Path::new(path!("/other/.git")), false, |state| {
        assert_eq!(state.stash_entries.entries.len(), 3);
    })
    .unwrap();
    fs.with_git_state(Path::new(path!("/project/.git")), false, |state| {
        assert_eq!(state.stash_entries.entries.len(), 3);
    })
    .unwrap();
}

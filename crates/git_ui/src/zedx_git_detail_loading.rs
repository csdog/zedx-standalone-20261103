use super::*;

#[derive(Default)]
pub(super) struct DetailsCache {
    sha: Option<Oid>,
    entries: Vec<ChangedFileEntry>,
    expanded: HashMap<RepoPath, bool>,
    tree: Rc<Vec<ChangedFileTreeEntry>>,
}

impl DetailsCache {
    pub(super) fn tree(
        &mut self,
        sha: Oid,
        diff: Option<&CommitDiff>,
        expanded: &HashMap<RepoPath, bool>,
        cx: &App,
    ) -> (usize, Rc<Vec<ChangedFileTreeEntry>>) {
        let Some(diff) = diff else {
            *self = Self::default();
            return (0, self.tree.clone());
        };
        let changed = self.sha != Some(sha) || self.entries.len() != diff.files.len();
        if changed {
            self.sha = Some(sha);
            self.entries = diff
                .files
                .iter()
                .map(|file| ChangedFileEntry::from_commit_file(file, cx))
                .collect();
        }
        if changed || &self.expanded != expanded {
            self.expanded = expanded.clone();
            self.tree = Rc::new(build_changed_file_tree_entries(
                self.entries.clone(),
                expanded,
            ));
        }
        (self.entries.len(), self.tree.clone())
    }
}

impl GitGraph {
    pub(super) fn load_zedx_selected_commit_diff(
        &mut self,
        sha: Oid,
        repository: &Entity<Repository>,
        cx: &mut Context<Self>,
    ) {
        let task = repository.update(cx, |repo, cx| {
            repo.load_commit_diff(sha.to_string(), false, cx)
        });
        self._commit_diff_task = Some(cx.spawn(async move |this, cx| {
            match task.await {
                Ok(diff) => {
                    this.update(cx, |this, cx| {
                        if this
                            .selected_entry_idx
                            .and_then(|index| this.graph_data.commits.get(index))
                            .is_none_or(|commit| commit.data.sha != sha)
                        {
                            return;
                        }
                        // ZedX shows paths/status here, so full line-diff statistics
                        // would spend UI-thread time on data the view never displays.
                        *this.zedx_details_cache.borrow_mut() = DetailsCache::default();
                        this.selected_commit_diff = Some(diff);
                        this.selected_commit_diff_stats = None;
                        cx.notify();
                    })
                    .log_err();
                }
                Err(error) => log::error!("Loading selected commit {sha}: {error:#}"),
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn repaint_reuses_tree_and_directory_toggle_invalidates_it(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let sha = Oid::from_bytes(&[1; 20]).unwrap();
            let diff = CommitDiff {
                files: ["src/a.rs", "src/b.rs"]
                    .into_iter()
                    .map(|path| CommitFile {
                        path: RepoPath::new(path).unwrap(),
                        old_text: None,
                        new_text: Some("fn example() {}".into()),
                        is_binary: false,
                    })
                    .collect(),
                is_shallow_boundary: false,
            };
            let mut cache = DetailsCache::default();
            let mut expanded = HashMap::default();
            let (count, first) = cache.tree(sha, Some(&diff), &expanded, cx);
            let (_, repaint) = cache.tree(sha, Some(&diff), &expanded, cx);
            assert_eq!(count, 2);
            assert!(Rc::ptr_eq(&first, &repaint));
            expanded.insert(RepoPath::new("src").unwrap(), false);
            let (_, collapsed) = cache.tree(sha, Some(&diff), &expanded, cx);
            assert!(!Rc::ptr_eq(&first, &collapsed));
            assert!(collapsed.len() < first.len());
            let (count, empty) = cache.tree(sha, None, &expanded, cx);
            assert_eq!(count, 0);
            assert!(empty.is_empty());
        });
    }
}

use super::policy::FileTypes;
use fuzzy_nucleo::{PathMatchCandidate, PathMatchCandidateSet};
use project::PathMatchCandidateSet as ProjectCandidates;
use std::sync::Arc;
use util::{paths::PathStyle, rel_path::RelPath};

pub(super) struct Candidates {
    source: ProjectCandidates,
    filtered: Option<Vec<Arc<RelPath>>>,
}

impl Candidates {
    pub(super) fn new(source: ProjectCandidates, filter: FileTypes) -> Self {
        // Filter before top-N ranking: filtering the first 100 matches can hide all matching files.
        let prefix = source.prefix();
        let filtered = (!filter.is_all()).then(|| {
            source
                .snapshot
                .files(source.include_ignored, 0)
                .filter(|entry| {
                    filter.accepts(if entry.path.is_empty() {
                        prefix.as_unix_str()
                    } else {
                        entry.path.as_unix_str()
                    })
                })
                .map(|entry| entry.path.clone())
                .collect()
        });
        Self { source, filtered }
    }
}

impl<'a> PathMatchCandidateSet<'a> for Candidates {
    type Candidates = Box<dyn Iterator<Item = PathMatchCandidate<'a>> + 'a>;

    fn id(&self) -> usize {
        self.source.id()
    }
    fn len(&self) -> usize {
        self.filtered
            .as_ref()
            .map_or_else(|| self.source.len(), Vec::len)
    }
    fn root_is_file(&self) -> bool {
        self.source.root_is_file()
    }
    fn prefix(&self) -> Arc<RelPath> {
        self.source.prefix()
    }
    fn path_style(&self) -> PathStyle {
        self.source.path_style()
    }
    fn candidates(&'a self, start: usize) -> Self::Candidates {
        match &self.filtered {
            Some(paths) => Box::new(
                paths
                    .iter()
                    .skip(start)
                    .map(|path| PathMatchCandidate::new(path, false, Some(&self.source.prefix()))),
            ),
            None => Box::new(self.source.candidates(start)),
        }
    }
}
